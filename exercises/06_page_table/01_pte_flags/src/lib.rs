//! # 页表项标志位（PTE Flags）
//!
//! 在这个练习中，你将学习 RISC-V SV39 页表项（PTE）的结构，
//! 并通过位操作构造/解析页表项。
//!
//! ## 概念
//! - RISC-V SV39 页表项 64 位布局
//! - 位操作构造/提取字段
//! - PTE 标志位（V/R/W/X/U/G/A/D）的含义
//!
//! ## SV39 PTE 布局（64 位）
//! ```text
//! 63    54 53        10 9  8 7 6 5 4 3 2 1 0
//! ┌───────┬────────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐
//! │ 保留  │  PPN[2:0]  │ RSW│D│A│G│U│X│W│R│V│
//! │ 10bit │   44 bits  │ 2b │ │ │ │ │ │ │ │ │
//! └───────┴────────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘
//! ```
//! - V（Valid，有效位）：指示该页表项是否有效。
//!
//! - R/W/X（Read/Write/Execute）：读、写、执行权限位。
//!
//! - U（User）：用户可访问位，允许用户态访问该页。
//!
//! - G（Global）：全局映射位（通常用于内核空间，避免 TLB 刷新）。
//!
//! - A（Accessed，访问位）：页被访问时由硬件置位。
//!
//! - D（Dirty，脏位）：页被写入时由硬件置位。
//!
//! - RSW（Reserved for Supervisor Software）：留给操作系统软件使用的两位。
//!
//! - PPN（Physical Page Number，物理页号）：占 44 位（bits [53:10]），指定物理页帧的基地址。
//! - PPN[2:0]：在 RISC-V SV39 分页机制中，物理页号（PPN）分为三部分，称为 PPN[2]、PPN[1] 和 PPN[0]。
//!   这样划分是为了支持多级页表的索引。
//! - Rsvd（Reserved，保留位）：保留位，通常置 0。

/// PTE 标志位常量
pub const PTE_V: u64 = 1 << 0; // 有效（Valid）
pub const PTE_R: u64 = 1 << 1; // 可读（Readable）
pub const PTE_W: u64 = 1 << 2; // 可写（Writable）
pub const PTE_X: u64 = 1 << 3; // 可执行（Executable）
pub const PTE_U: u64 = 1 << 4; // 用户可访问（User accessible）
pub const PTE_G: u64 = 1 << 5; // 全局（Global）
pub const PTE_A: u64 = 1 << 6; // 已访问（Accessed）
pub const PTE_D: u64 = 1 << 7; // 脏（Dirty）

/// PPN 字段在 PTE 中的偏移与掩码
const PPN_SHIFT: u32 = 10;
const PPN_MASK: u64 = (1u64 << 44) - 1; // 44 位 PPN

/// 由物理页号（PPN）和标志位构造页表项。
///
/// PPN 占 bits [53:10]，标志位占 bits [7:0]。
///
/// 示例：ppn=0x12345, flags=PTE_V|PTE_R|PTE_W
/// 结果应为：(0x12345 << 10) | 0b111 = 0x48D14007
///
/// 提示：把 PPN 左移 PPN_SHIFT 位，然后与 flags 按位或。
pub fn make_pte(ppn: u64, flags: u64) -> u64 {
    // TODO：用 ppn 和 flags 构造页表项
    (ppn << PPN_SHIFT) | flags
}

/// 从页表项中提取物理页号（PPN）。
///
/// 提示：右移 PPN_SHIFT 位，然后与 PPN_MASK 按位与。
pub fn extract_ppn(pte: u64) -> u64 {
    // TODO：从 pte 中提取 PPN
    pte>>10&PPN_MASK
}

/// 从页表项中提取标志位（低 8 位）。
pub fn extract_flags(pte: u64) -> u64 {
    // TODO：提取低 8 位标志
    pte&0xff
}

/// 检查页表项是否有效（V 位已置位）。
pub fn is_valid(pte: u64) -> bool {
    // TODO：检查 PTE_V
    
    (pte&PTE_V)!=0
}

/// 判断页表项是否是叶节点（leaf PTE）。
///
/// 在 SV39 中，只要 R、W、X 任一位置位，该 PTE 就是叶节点，
/// 指向最终的物理页；否则它指向下一级页表。
pub fn is_leaf(pte: u64) -> bool {
    // TODO：检查 R/W/X 是否有任一位置位
    ((pte&PTE_R)|(pte&PTE_W)|(pte&PTE_X))!= 0
}

/// 根据请求的访问类型，检查页表项是否允许该访问。
///
/// - `read`：需要读权限
/// - `write`：需要写权限
/// - `execute`：需要执行权限
///
/// 当且仅当：PTE 有效，且每个请求的权限都满足时，返回 true。
pub fn check_permission(pte: u64, read: bool, write: bool, execute: bool) -> bool {
    // TODO：先检查有效性，再逐项检查请求的权限
      if !is_valid(pte) {
        return false;
    }
    if read && (pte & PTE_R) == 0 {
        return false;
    }
    if write && (pte & PTE_W) == 0 {
        return false;
    }
    if execute && (pte & PTE_X) == 0 {
        return false;
    }
    true
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_pte_basic() {
        let pte = make_pte(0x12345, PTE_V | PTE_R | PTE_W);
        assert_eq!(extract_ppn(pte), 0x12345);
        assert_eq!(extract_flags(pte), PTE_V | PTE_R | PTE_W);
    }

    #[test]
    fn test_make_pte_zero() {
        let pte = make_pte(0, 0);
        assert_eq!(pte, 0);
        assert_eq!(extract_ppn(pte), 0);
        assert_eq!(extract_flags(pte), 0);
    }

    #[test]
    fn test_make_pte_all_flags() {
        let all = PTE_V | PTE_R | PTE_W | PTE_X | PTE_U | PTE_G | PTE_A | PTE_D;
        let pte = make_pte(0xABC, all);
        assert_eq!(extract_ppn(pte), 0xABC);
        assert_eq!(extract_flags(pte), all);
    }

    #[test]
    fn test_make_pte_large_ppn() {
        let ppn = (1u64 << 44) - 1; // 最大 PPN
        let pte = make_pte(ppn, PTE_V);
        assert_eq!(extract_ppn(pte), ppn);
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid(make_pte(1, PTE_V)));
        assert!(!is_valid(make_pte(1, PTE_R))); // R 置位但 V 未置位
        assert!(!is_valid(0));
    }

    #[test]
    fn test_is_leaf() {
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_X)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X)));
        // 非叶节点：只置 V，R/W/X 全清零
        assert!(!is_leaf(make_pte(1, PTE_V)));
        assert!(!is_leaf(make_pte(1, PTE_V | PTE_A | PTE_D)));
    }

    #[test]
    fn test_check_permission_read() {
        let pte = make_pte(1, PTE_V | PTE_R);
        assert!(check_permission(pte, true, false, false));
        assert!(!check_permission(pte, false, true, false));
        assert!(!check_permission(pte, false, false, true));
    }

    #[test]
    fn test_check_permission_rw() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W);
        assert!(check_permission(pte, true, true, false));
        assert!(!check_permission(pte, true, true, true));
    }

    #[test]
    fn test_check_permission_all() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X);
        assert!(check_permission(pte, true, true, true));
        assert!(check_permission(pte, true, false, false));
        assert!(check_permission(pte, false, false, false)); // 无要求 = 允许
    }

    #[test]
    fn test_check_permission_invalid() {
        // V 未置位，即使 R/W/X 都在，也应返回 false
        let pte = make_pte(1, PTE_R | PTE_W | PTE_X);
        assert!(!check_permission(pte, true, false, false));
    }
}
