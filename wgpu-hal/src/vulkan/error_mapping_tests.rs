//! Checks how `VkResult` codes map to [`crate::DeviceError`] and how unexpected codes are
//! described in the log.

use alloc::{format, string::ToString};

use ash::vk;

use super::UnexpectedVkResult;

#[test]
fn unexpected_result_names_call_code_and_label() {
    let report = UnexpectedVkResult {
        err: vk::Result::ERROR_UNKNOWN,
        call: "vkCreateGraphicsPipelines",
        label: Some("terrain"),
    };
    assert_eq!(
        report.to_string(),
        "vkCreateGraphicsPipelines returned unexpected ERROR_UNKNOWN (-13) for \"terrain\""
    );
}

#[test]
fn unexpected_result_without_label_omits_it() {
    let report = UnexpectedVkResult {
        err: vk::Result::ERROR_INITIALIZATION_FAILED,
        call: "vkCreateShaderModule",
        label: None,
    };
    assert_eq!(
        report.to_string(),
        "vkCreateShaderModule returned unexpected ERROR_INITIALIZATION_FAILED (-3)"
    );
}

#[test]
fn unexpected_result_keeps_codes_unknown_to_ash() {
    let report = UnexpectedVkResult {
        err: vk::Result::from_raw(-1_234_567),
        call: "vkQueueSubmit",
        label: None,
    };
    assert!(
        format!("{report}").ends_with("(-1234567)"),
        "the raw value must survive: {report}"
    );
}

#[test]
fn out_of_memory_codes_stay_out_of_memory() {
    for err in [
        vk::Result::ERROR_OUT_OF_HOST_MEMORY,
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
    ] {
        assert_eq!(
            super::map_host_device_oom_and_lost_err(err, "vkQueueSubmit"),
            crate::DeviceError::OutOfMemory
        );
    }
}

#[test]
fn device_lost_stays_lost() {
    #[cfg(not(feature = "device_lost_panic"))]
    assert_eq!(
        super::map_host_device_oom_and_lost_err(vk::Result::ERROR_DEVICE_LOST, "vkQueueSubmit"),
        crate::DeviceError::Lost
    );
}

#[cfg(not(feature = "internal_error_panic"))]
#[test]
fn other_codes_are_unexpected() {
    assert_eq!(
        super::map_host_device_oom_err(vk::Result::ERROR_DEVICE_LOST, "vkCreateBuffer"),
        crate::DeviceError::Unexpected
    );
}
