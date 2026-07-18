//! Inventory window + slot logic. Lands in M8.

#[derive(Debug, Clone)]
pub struct ItemStack {
    pub item_id: u32,
    pub count: u8,
}
