use crate::util::HasBuilder;

impl HasBuilder for vk_mem::AllocationCreateInfo {
    type Builder = AllocationCreateInfoBuilder;
    fn builder() -> Self::Builder {
        AllocationCreateInfoBuilder {
            inner: vk_mem::AllocationCreateInfo::default()
        }
    }
}

pub struct AllocationCreateInfoBuilder {
    inner: vk_mem::AllocationCreateInfo,
}

impl AllocationCreateInfoBuilder {

    pub fn usage(mut self, usage: vk_mem::MemoryUsage) -> AllocationCreateInfoBuilder {
        self.inner.usage = usage;
        self
    }

    pub fn flags(mut self, flags: vk_mem::AllocationCreateFlags) -> AllocationCreateInfoBuilder {
        self.inner.flags = flags;
        self
    }

    pub fn required_flags(mut self, required_flags: ash::vk::MemoryPropertyFlags) -> AllocationCreateInfoBuilder {
        self.inner.required_flags = required_flags;
        self
    }

    pub fn preferred_flags(mut self, preferred_flags: ash::vk::MemoryPropertyFlags) -> AllocationCreateInfoBuilder {
        self.inner.preferred_flags = preferred_flags;
        self
    }

    pub fn memory_type_bits(mut self, memory_type_bits: u32) -> AllocationCreateInfoBuilder {
        self.inner.memory_type_bits = memory_type_bits;
        self
    }

    pub fn pool(mut self, pool: vk_mem::AllocatorPool) -> AllocationCreateInfoBuilder {
        self.inner.pool = Some(pool);
        self
    }

    pub fn user_data(mut self, user_data: *mut ::std::os::raw::c_void) -> AllocationCreateInfoBuilder {
        self.inner.user_data = Some(user_data);
        self
    }

    pub fn build(self) -> vk_mem::AllocationCreateInfo {
        self.inner
    }
}
