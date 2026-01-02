// @generated automatically by Diesel CLI.

diesel::table! {
    clipboard (id) {
        id -> Nullable<Integer>,
        text -> Nullable<Text>,
        image -> Nullable<Text>,
        image_width -> Nullable<Integer>,
        image_height -> Nullable<Integer>,
        timestamp -> Nullable<Integer>,
        size_bytes -> Nullable<Integer>,
        source_app -> Nullable<Text>,
    }
}

diesel::table! {
    items (id) {
        id -> Integer,
        text -> Nullable<Text>,
        image -> Nullable<Text>,
        image_width -> Nullable<Integer>,
        image_height -> Nullable<Integer>,
        timestamp -> BigInt,
        size_bytes -> Integer,
        source_app -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(clipboard, items,);
