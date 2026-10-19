//! This file describes "special cases" that are used to supplement the automatic wrapper generation
//! in cases where we want to provide a "nicer" API.
//!
//! - `#[skip]` = Skip the generation of a wrapper.
//! - `#[replace]` = Replace the arguments, return type, and content of a wrapper.
//!
//! Note that the command name must match the original one in vk.xml,
//! the generator will adjust it on its own accord.

^ actually, it should all just be replace..... wip todo


// Defined manually as part of the loader setup.
#[skip]
fn vkGetInstanceProcAddr();
#[skip]
fn vkGetDeviceProcAddr();
#[skip]
fn vkEnumerateInstanceVersion();
#[skip]
fn vkCreateInstance();
#[skip]
fn vkCreateDevice();

// TODO: remove once these are fixed
#[skip]
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR();
#[skip]
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceCountersByRegionARM();
#[skip]
fn vkGetPipelineBinaryDataKHR();
#[skip]
fn vkGetEncodedVideoSessionParametersKHR();
