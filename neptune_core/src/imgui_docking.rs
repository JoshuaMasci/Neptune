pub fn enable_docking() {
    unsafe {
        let _ = imgui::sys::igDockSpaceOverViewport(
            imgui::sys::igGetMainViewport(),
            imgui::sys::ImGuiDockNodeFlags_PassthruCentralNode as imgui::sys::ImGuiDockNodeFlags,
            std::ptr::null(),
        );
    }
}
