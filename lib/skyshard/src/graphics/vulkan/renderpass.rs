use crate::graphics::vulkan::device::DeviceRef;
use crate::graphics::vulkan::surface::SurfaceRef;
use ash::version::DeviceV1_0;

pub fn create_render_pass(device: DeviceRef, surface: SurfaceRef) -> ash::vk::RenderPass {

    let _device = (*device).borrow();
    let _surface = (*surface).borrow();
    let surface_formats = _surface.get_formats(_device.physical_device()).unwrap();
    let surface_format = surface_formats.first().unwrap();

    let attachments = [
        ash::vk::AttachmentDescription::builder()
            .format(surface_format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build(),
    ];

    let color_attachment_refs = [ash::vk::AttachmentReference {
        attachment: 0,
        layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    // let depth_attachment_ref = ash::vk::AttachmentReference {
    //     attachment: 1,
    //     layout: ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    // };

    let dependencies = [
        ash::vk::SubpassDependency::builder()
            .src_subpass(ash::vk::SUBPASS_EXTERNAL)
            .src_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(ash::vk::AccessFlags::default())
            .dst_subpass(0)
            .dst_stage_mask(ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build(),
    ];

    let subpasses = [
        ash::vk::SubpassDescription::builder()
            .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)
            // .depth_stencil_attachment(&depth_attachment_ref)
            .build(),
    ];

    let create_info = ash::vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let renderpass = unsafe {
        _device.handle().create_render_pass(&create_info, None)
    }.unwrap();

    renderpass
}
