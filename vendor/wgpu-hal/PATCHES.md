# wgpu-hal 30.0.1, patched

This is wgpu-hal 30.0.1 as published on crates.io (`.cargo_vcs_info.json` names its upstream
commit), used through `[patch.crates-io]` in the workspace `Cargo.toml`, with these changes:

- **Full-screen exclusive mode lost** (`src/vulkan/swapchain/native.rs`). Upstream maps a
  `vkAcquireNextImageKHR` or `vkQueuePresentKHR` result it does not handle to
  `DeviceError::Unexpected`, which wgpu-core takes for a lost device, so every later frame fails.
  `VK_ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT` now gives `SurfaceError::Outdated`, as
  `VK_ERROR_OUT_OF_DATE_KHR` does, and the swapchain is configured again. After a present that
  returns it, the next acquire returns `Outdated` without calling `vkAcquireNextImageKHR`.
- **An acquire fence that times out** (`src/vulkan/swapchain/native.rs`). On Windows, after
  `vkAcquireNextImageKHR` returns an image, upstream waits for the acquire fence with the
  acquire's timeout (wgpu-core passes one second) and maps any error to a device error. A wait
  that times out returns `VK_TIMEOUT`, which `ash` gives as an error and upstream maps to
  `DeviceError::Unexpected` ("Unexpected error variant (driver implementation is at fault)"),
  so wgpu-core loses the device. A covered or minimized window's image can be held that long.
  Now the image is handed out all the same (the first submission that uses it waits on its
  acquire semaphore) and the fence stays pending: the next acquire waits for it first and
  returns `SurfaceError::Timeout` while it has not signaled, without acquiring an image. A
  swapchain released while the fence is pending gives it up to a second before destroying it.
- **The result behind a lost device** (`src/vulkan/mod.rs`, `src/lib.rs`).
  `last_unexpected_vulkan_result()` names the last Vulkan result the backend did not handle
  (`get_unexpected_err`, and an acquire's or a present's), which upstream maps to
  `DeviceError::Unexpected` and wgpu-core to a lost device. The game's error for a lost device
  shows it.

wgpu-hal is licensed MIT OR Apache-2.0 (`LICENSE.MIT`, `LICENSE.APACHE`).
