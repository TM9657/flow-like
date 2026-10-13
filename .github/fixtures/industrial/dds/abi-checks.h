#include <stddef.h>
#include "dds/ddsc/dds_public_dynamic_type.h"
// Rust has matching compile-time assertions in dds/ffi.rs. All supported desktops use this 64-bit ABI.
#define SIZE(type, size) _Static_assert(sizeof(type) == (size), #type " ABI size")
#define OFFSET(type, field, offset) _Static_assert(offsetof(type, field) == (offset), #type "." #field " ABI offset")
SIZE(void *, 8);
SIZE(dds_dynamic_type_t, 24); OFFSET(dds_dynamic_type_t, ret, 16);
SIZE(dds_dynamic_type_spec_t, 32); OFFSET(dds_dynamic_type_spec_t, type, 8);
SIZE(dds_dynamic_type_descriptor_t, 160);
OFFSET(dds_dynamic_type_descriptor_t, name, 8); OFFSET(dds_dynamic_type_descriptor_t, base_type, 16);
OFFSET(dds_dynamic_type_descriptor_t, discriminator_type, 48); OFFSET(dds_dynamic_type_descriptor_t, num_bounds, 80);
OFFSET(dds_dynamic_type_descriptor_t, bounds, 88); OFFSET(dds_dynamic_type_descriptor_t, element_type, 96);
OFFSET(dds_dynamic_type_descriptor_t, key_element_type, 128);
SIZE(dds_dynamic_member_descriptor_t, 80); OFFSET(dds_dynamic_member_descriptor_t, id, 8);
OFFSET(dds_dynamic_member_descriptor_t, type, 16); OFFSET(dds_dynamic_member_descriptor_t, default_value, 48);
OFFSET(dds_dynamic_member_descriptor_t, index, 56); OFFSET(dds_dynamic_member_descriptor_t, num_labels, 60);
OFFSET(dds_dynamic_member_descriptor_t, labels, 64); OFFSET(dds_dynamic_member_descriptor_t, default_label, 72);
OFFSET(dds_topic_descriptor_t, m_size, 0); OFFSET(dds_topic_descriptor_t, m_align, 4);
OFFSET(dds_topic_descriptor_t, m_flagset, 8); OFFSET(dds_topic_descriptor_t, m_nkeys, 12);
OFFSET(dds_topic_descriptor_t, m_typename, 16); OFFSET(dds_topic_descriptor_t, m_keys, 24);
OFFSET(dds_topic_descriptor_t, m_nops, 32); OFFSET(dds_topic_descriptor_t, m_ops, 40);
OFFSET(dds_topic_descriptor_t, m_meta, 48);
SIZE(dds_sample_info_t, 64); OFFSET(dds_sample_info_t, valid_data, 12);
OFFSET(dds_sample_info_t, source_timestamp, 16); OFFSET(dds_sample_info_t, instance_handle, 24);
OFFSET(dds_sample_info_t, publication_handle, 32); OFFSET(dds_sample_info_t, disposed_generation_count, 40);
OFFSET(dds_sample_info_t, no_writers_generation_count, 44); OFFSET(dds_sample_info_t, sample_rank, 48);
OFFSET(dds_sample_info_t, generation_rank, 52); OFFSET(dds_sample_info_t, absolute_generation_rank, 56);
SIZE(dds_sequence_short, 24); OFFSET(dds_sequence_short, _maximum, 0); OFFSET(dds_sequence_short, _length, 4);
OFFSET(dds_sequence_short, _buffer, 8); OFFSET(dds_sequence_short, _release, 16);
#undef SIZE
#undef OFFSET
