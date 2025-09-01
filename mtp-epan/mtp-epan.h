#pragma once

#include <stdint.h>

#define PARSED_CONTAINER_FIELD_STR(field_name) \
	const char* field_name; \
	int         field_name##_start; \
	int         field_name##_type_len; \
	int         field_name##_str_len;

#define PARSED_CONTAINER_FIELD_INT(field_name) \
	uint32_t	field_name; \
	int         field_name##_start; \
	int         field_name##_len;

#define PARSED_CONTAINER_FIELDS(STR_MACRO, INT_MACRO) \
	INT_MACRO(container_len) \
	STR_MACRO(container_type) \
	STR_MACRO(opcode) \
	INT_MACRO(transaction_id) \

#define PARSED_CONTAINER() \
typedef struct { \
PARSED_CONTAINER_FIELDS(PARSED_CONTAINER_FIELD_STR, PARSED_CONTAINER_FIELD_INT) \
} parsed_container;

PARSED_CONTAINER()

#undef PARSED_CONTAINER_FIELD_STR
#undef PARSED_CONTAINER_FIELD_INT
#undef PARSED_CONTAINER
