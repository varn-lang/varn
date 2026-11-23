use vn_sem::semantic_info::ResolvedMemberKind;
use vn_sem::types::ClassMemberKind;

pub(super) fn map_class_member_kind(k: ClassMemberKind) -> ResolvedMemberKind {
    match k {
        ClassMemberKind::Method | ClassMemberKind::Function => ResolvedMemberKind::Method,
        ClassMemberKind::Property | ClassMemberKind::Variable => ResolvedMemberKind::Property,
        ClassMemberKind::Constructor => ResolvedMemberKind::Constructor,
        ClassMemberKind::Getter => ResolvedMemberKind::Getter,
        ClassMemberKind::Setter => ResolvedMemberKind::Setter,
        ClassMemberKind::Type(k) => ResolvedMemberKind::NestedType(k),
    }
}
