pub fn enable_docking() -> imgui::sys::ImGuiID {
    unsafe {
        imgui::sys::igDockSpaceOverViewport(
            imgui::sys::igGetMainViewport(),
            imgui::sys::ImGuiDockNodeFlags_PassthruCentralNode as imgui::sys::ImGuiDockNodeFlags,
            std::ptr::null(),
        )
    }
}

pub fn try_dock(id: imgui::sys::ImGuiID) {
    unsafe {
        imgui::sys::igSetNextWindowDockID(
            id,
            imgui::sys::ImGuiCond_Always as imgui::sys::ImGuiCond,
        );
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    None = imgui::sys::ImGuiDir_None,
    Left = imgui::sys::ImGuiDir_Left,
    Right = imgui::sys::ImGuiDir_Right,
    Up = imgui::sys::ImGuiDir_Up,
    Down = imgui::sys::ImGuiDir_Down,
}

#[derive(Clone, Copy)]
pub struct DockNode<'ui> {
    ui: &'ui imgui::Ui,
    id: imgui::sys::ImGuiID,
}

impl<'ui> DockNode<'ui> {
    pub fn new(ui: &'ui imgui::Ui, id: imgui::sys::ImGuiID) -> Self {
        Self { ui, id }
    }

    pub fn size(self, size: [f32; 2]) -> Self {
        unsafe { imgui::sys::igDockBuilderSetNodeSize(self.id, imgui::sys::ImVec2::from(size)) }

        self
    }

    pub fn position(self, position: [f32; 2]) -> Self {
        unsafe { imgui::sys::igDockBuilderSetNodePos(self.id, imgui::sys::ImVec2::from(position)) }

        self
    }

    pub fn dock_window<Label: AsRef<str>>(self, window_name: Label) -> Self {
        let scratch_text = unsafe {
            let handle = self.ui.scratch_buffer().get();
            (*handle).scratch_txt(window_name)
        };
        unsafe { imgui::sys::igDockBuilderDockWindow(scratch_text, self.id) }
        self
    }

    pub fn split<D: FnOnce(DockNode), O: FnOnce(DockNode)>(
        self,
        split_dir: Direction,
        size_ratio: f32,
        dir: D,
        opposite_dir: O,
    ) {
        let mut out_id_at_dir: imgui::sys::ImGuiID = 0;
        let mut out_id_at_opposite_dir: imgui::sys::ImGuiID = 0;

        unsafe {
            imgui::sys::igDockBuilderSplitNode(
                self.id,
                split_dir as i32,
                size_ratio,
                &mut out_id_at_dir,
                &mut out_id_at_opposite_dir,
            );
        }

        dir(DockNode::new(self.ui, out_id_at_dir));
        opposite_dir(DockNode::new(self.ui, out_id_at_opposite_dir));
    }
}

pub struct Dock<'ui> {
    ui: &'ui imgui::Ui,
}

impl<'ui> Dock<'ui> {
    //TODO: pass Ui into here
    pub fn new(ui: &'ui imgui::Ui) -> Self {
        Self { ui }
    }

    pub fn build<F: FnOnce(DockNode)>(self, f: F) {
        let dock_id = unsafe {
            imgui::sys::igDockBuilderAddNode(0, imgui::sys::ImGuiDockNodeFlags_None as i32)
        };

        f(DockNode::new(self.ui, dock_id));

        unsafe { imgui::sys::igDockBuilderFinish(dock_id) }
    }
}
