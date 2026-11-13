//! Checks which `VkResult` codes from shader-module and pipeline creation are fatal to the
//! device, and what the non-fatal ones report.

#[cfg(not(feature = "internal_error_panic"))]
use alloc::string::ToString;

use ash::vk;

use super::{map_pipeline_err, PipelineCreationError};

/// Codes a driver returns when it cannot build one pipeline or shader module.
#[cfg(not(feature = "internal_error_panic"))]
const DRIVER_FAILURES: [vk::Result; 4] = [
    vk::Result::ERROR_UNKNOWN,
    vk::Result::ERROR_INITIALIZATION_FAILED,
    vk::Result::ERROR_INVALID_SHADER_NV,
    vk::Result::ERROR_FEATURE_NOT_PRESENT,
];

#[cfg(not(feature = "internal_error_panic"))]
#[test]
fn driver_failures_are_not_device_errors() {
    for result in DRIVER_FAILURES {
        for call in [
            "vkCreateGraphicsPipelines",
            "vkCreateComputePipelines",
            "vkCreateShaderModule",
        ] {
            assert_eq!(
                map_pipeline_err(result, call),
                PipelineCreationError::Driver { call, result }
            );
        }
    }
}

#[cfg(not(feature = "internal_error_panic"))]
#[test]
fn pipeline_driver_failure_is_a_linkage_error_naming_call_code_and_label() {
    let error = map_pipeline_err(vk::Result::ERROR_UNKNOWN, "vkCreateGraphicsPipelines")
        .into_pipeline_error(wgt::ShaderStages::VERTEX_FRAGMENT, Some("terrain"));
    assert_eq!(
        error,
        crate::PipelineError::Linkage(
            wgt::ShaderStages::VERTEX_FRAGMENT,
            "vkCreateGraphicsPipelines failed with ERROR_UNKNOWN (-13) for pipeline \"terrain\""
                .to_string()
        )
    );

    let error = map_pipeline_err(
        vk::Result::ERROR_INITIALIZATION_FAILED,
        "vkCreateShaderModule",
    )
    .into_pipeline_error(wgt::ShaderStages::FRAGMENT, None);
    assert_eq!(
        error,
        crate::PipelineError::Linkage(
            wgt::ShaderStages::FRAGMENT,
            "vkCreateShaderModule failed with ERROR_INITIALIZATION_FAILED (-3) for an unlabeled \
             pipeline"
                .to_string()
        )
    );
}

#[cfg(not(feature = "internal_error_panic"))]
#[test]
fn shader_module_driver_failure_is_a_compilation_error_naming_call_code_and_label() {
    let error = map_pipeline_err(vk::Result::ERROR_INVALID_SHADER_NV, "vkCreateShaderModule")
        .into_shader_error(Some("sky"));
    assert_eq!(
        error,
        crate::ShaderError::Compilation(
            "vkCreateShaderModule failed with ERROR_INVALID_SHADER_NV (-1000012000) for shader \
             module \"sky\""
                .to_string()
        )
    );
}

#[cfg(not(feature = "internal_error_panic"))]
#[test]
fn driver_failure_keeps_codes_unknown_to_ash() {
    let error = map_pipeline_err(vk::Result::from_raw(-1_234_567), "vkCreateComputePipelines")
        .into_pipeline_error(wgt::ShaderStages::COMPUTE, Some("cull"));
    let crate::PipelineError::Linkage(_, message) = error else {
        panic!("expected a linkage error, got {error:?}");
    };
    assert!(message.contains("(-1234567)"), "{message}");
}

#[test]
fn out_of_memory_stays_a_device_error() {
    for result in [
        vk::Result::ERROR_OUT_OF_HOST_MEMORY,
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
    ] {
        let error = map_pipeline_err(result, "vkCreateGraphicsPipelines");
        assert_eq!(
            error,
            PipelineCreationError::Device(crate::DeviceError::OutOfMemory)
        );
        assert_eq!(
            error.into_pipeline_error(wgt::ShaderStages::VERTEX_FRAGMENT, Some("terrain")),
            crate::PipelineError::Device(crate::DeviceError::OutOfMemory)
        );
    }
    assert_eq!(
        map_pipeline_err(vk::Result::ERROR_OUT_OF_HOST_MEMORY, "vkCreateShaderModule")
            .into_shader_error(None),
        crate::ShaderError::Device(crate::DeviceError::OutOfMemory)
    );
}

#[cfg(not(feature = "device_lost_panic"))]
#[test]
fn device_lost_stays_a_device_error() {
    let error = map_pipeline_err(vk::Result::ERROR_DEVICE_LOST, "vkCreateGraphicsPipelines");
    assert_eq!(
        error,
        PipelineCreationError::Device(crate::DeviceError::Lost)
    );
    assert_eq!(
        error.into_pipeline_error(wgt::ShaderStages::VERTEX_FRAGMENT, Some("terrain")),
        crate::PipelineError::Device(crate::DeviceError::Lost)
    );
    assert_eq!(
        map_pipeline_err(vk::Result::ERROR_DEVICE_LOST, "vkCreateShaderModule")
            .into_shader_error(Some("sky")),
        crate::ShaderError::Device(crate::DeviceError::Lost)
    );
}
