//! Tests of how a pipeline that the backend fails to create is reported: a driver that
//! cannot build one pipeline must not cost the application its device.

use std::sync::Arc;

use parking_lot::Mutex;
use wgpu::hal::noop::PipelineFailure;
use wgpu_test::{fail, valid};

const RENDER_SHADER: &str = "\
@group(0) @binding(0) var<uniform> tint: vec4f;

@vertex
fn vert() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn frag() -> @location(0) vec4f {
    return tint;
}
";

const COMPUTE_SHADER: &str = "\
@group(0) @binding(0) var<uniform> value: u32;

@compute @workgroup_size(1)
fn main() {
    _ = value;
}
";

/// Makes the noop backend fail the next pipeline creation on `device` with `failure`.
fn fail_next_pipeline_creation(device: &wgpu::Device, failure: PipelineFailure) {
    // SAFETY: the backend device is only told to fail, which touches no wgpu-owned state.
    let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Noop>() }
        .expect("the device should be a noop backend device");
    hal_device.fail_next_pipeline_creation(failure);
}

/// Records every call of the device-lost callback of `device`.
fn record_device_losses(
    device: &wgpu::Device,
) -> Arc<Mutex<Vec<(wgpu::DeviceLostReason, String)>>> {
    let losses = Arc::new(Mutex::new(Vec::new()));
    let recorded = losses.clone();
    device.set_device_lost_callback(move |reason, message| {
        recorded.lock().push((reason, message));
    });
    losses
}

/// Records every error `device` reports outside an error scope.
fn record_uncaptured_errors(device: &wgpu::Device) -> Arc<Mutex<Vec<wgpu::Error>>> {
    let errors = Arc::new(Mutex::new(Vec::new()));
    let recorded = errors.clone();
    device.on_uncaptured_error(Arc::new(move |error| {
        recorded.lock().push(error);
    }));
    errors
}

fn create_render_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terrain"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: None,
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: None,
            compilation_options: Default::default(),
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_compute_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cull"),
        layout: None,
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        cache: None,
    })
}

/// Asserts that `error` is the internal error of an injected driver failure.
#[track_caller]
fn assert_is_driver_failure(error: &wgpu::Error) {
    let wgpu::Error::Internal { description, .. } = error else {
        panic!("expected an internal error, got {error:?}");
    };
    assert!(
        description.contains(PipelineFailure::DRIVER_MESSAGE),
        "the driver's message should reach the application: {description}"
    );
}

#[test]
fn render_pipeline_driver_failure_is_an_internal_error_that_keeps_the_device() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let losses = record_device_losses(&device);
    let uncaptured = record_uncaptured_errors(&device);

    fail_next_pipeline_creation(&device, PipelineFailure::Driver);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let pipeline = create_render_pipeline(&device);
    let error = pollster::block_on(scope.pop()).expect("the failure should be reported");
    assert_is_driver_failure(&error);
    assert!(
        error.to_string().contains("terrain"),
        "the error should name the pipeline: {error}"
    );

    fail(
        &device,
        || pipeline.get_bind_group_layout(0),
        Some("RenderPipeline with 'terrain' label is invalid"),
    );

    // The device still works, so the application may retry or go without this pipeline.
    let retried = valid(&device, || create_render_pipeline(&device));
    valid(&device, || retried.get_bind_group_layout(0));
    assert_eq!(*losses.lock(), []);
    assert!(uncaptured.lock().is_empty());
}

#[test]
fn compute_pipeline_driver_failure_is_an_internal_error_that_keeps_the_device() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let losses = record_device_losses(&device);
    let uncaptured = record_uncaptured_errors(&device);

    fail_next_pipeline_creation(&device, PipelineFailure::Driver);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let pipeline = create_compute_pipeline(&device);
    let error = pollster::block_on(scope.pop()).expect("the failure should be reported");
    assert_is_driver_failure(&error);
    assert!(
        error.to_string().contains("cull"),
        "the error should name the pipeline: {error}"
    );

    fail(
        &device,
        || pipeline.get_bind_group_layout(0),
        Some("ComputePipeline with 'cull' label is invalid"),
    );

    let retried = valid(&device, || create_compute_pipeline(&device));
    valid(&device, || retried.get_bind_group_layout(0));
    assert_eq!(*losses.lock(), []);
    assert!(uncaptured.lock().is_empty());
}

#[test]
fn driver_failure_outside_an_error_scope_reaches_the_uncaptured_error_handler() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let losses = record_device_losses(&device);
    let uncaptured = record_uncaptured_errors(&device);

    fail_next_pipeline_creation(&device, PipelineFailure::Driver);
    let _pipeline = create_render_pipeline(&device);

    {
        let uncaptured = uncaptured.lock();
        let [error] = uncaptured.as_slice() else {
            panic!("expected one uncaptured error, got {uncaptured:?}");
        };
        assert_is_driver_failure(error);
    }
    assert_eq!(*losses.lock(), []);
}

#[test]
fn device_lost_during_pipeline_creation_loses_the_device() {
    for create in [
        (|device| drop(create_render_pipeline(device))) as fn(&wgpu::Device),
        |device| drop(create_compute_pipeline(device)),
    ] {
        let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let losses = record_device_losses(&device);
        let uncaptured = record_uncaptured_errors(&device);

        fail_next_pipeline_creation(&device, PipelineFailure::DeviceLost);
        create(&device);

        assert_eq!(
            *losses.lock(),
            [(
                wgpu::DeviceLostReason::Unknown,
                "Device is lost".to_string()
            )]
        );
        // A device loss is reported through the callback alone.
        assert!(uncaptured.lock().is_empty());
    }
}

#[test]
fn out_of_memory_during_pipeline_creation_loses_the_device() {
    for create in [
        (|device| drop(create_render_pipeline(device))) as fn(&wgpu::Device),
        |device| drop(create_compute_pipeline(device)),
    ] {
        let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let losses = record_device_losses(&device);

        fail_next_pipeline_creation(&device, PipelineFailure::OutOfMemory);
        let scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        create(&device);
        let error = pollster::block_on(scope.pop());

        assert!(
            matches!(error, Some(wgpu::Error::OutOfMemory { .. })),
            "expected an out-of-memory error, got {error:?}"
        );
        assert_eq!(
            *losses.lock(),
            [(wgpu::DeviceLostReason::Unknown, "Out of memory".to_string())]
        );
    }
}
