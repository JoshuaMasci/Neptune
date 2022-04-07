use crate::buffer::BufferId;

pub(crate) trait InstanceImpl {}
pub(crate) trait DeviceImpl {
    fn drop_buffer(&self, handle: BufferId);
}
