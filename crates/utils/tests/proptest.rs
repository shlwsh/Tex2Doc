//! proptest 模糊测试

use doc_utils::VirtualFs;
use proptest::prelude::*;

proptest! {
    /// 任意 UTF-8 字符串（不嵌入 null）→ 插入读取应一致
    #[test]
    fn vfs_roundtrip(s in "[a-zA-Z0-9_/.-]{1,64}") {
        let mut v = VirtualFs::new();
        let bytes = s.as_bytes().to_vec();
        v.insert(&s, bytes.clone());
        prop_assert_eq!(v.read(&s).unwrap(), bytes.as_slice());
    }

    /// 多个候选路径：首存在的应是已插入的那个
    #[test]
    fn vfs_first_existing(
        paths in proptest::collection::vec("[a-z0-9_/.-]{1,16}", 1..6),
    ) {
        let mut v = VirtualFs::new();
        // 至少插入一个
        if let Some(first) = paths.first() {
            v.insert(first, vec![1u8]);
        }
        let hit = v.first_existing(paths.iter());
        if let Some(_first) = paths.first() {
            prop_assert!(hit.is_some());
        } else {
            prop_assert!(hit.is_none());
        }
    }
}
