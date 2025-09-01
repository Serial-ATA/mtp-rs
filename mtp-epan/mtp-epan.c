#include "mtp-epan.h"
#include <epan/packet.h>
#include <gmodule.h>
#include <stdbool.h>
#include <stdint.h>

const char *PROTOCOL_NAME_FULL = "Media Transfer Protocol";
const char *PROTOCOL_NAME_SHORT = "MTP";
const char *PROTOCOL_FILTER_NAME = "mtp";

G_MODULE_EXPORT const gchar plugin_version[] = "0.0.1";
G_MODULE_EXPORT const int plugin_want_major = WIRESHARK_VERSION_MAJOR;
G_MODULE_EXPORT const int plugin_want_minor = WIRESHARK_VERSION_MINOR;

static int proto_mtp;
static int ett_mtp = -1;
static int *ett[] = {&ett_mtp};

static int hf_mtp_container_len = -1;
static int hf_mtp_container_type = -1;
static int hf_mtp_opcode = -1;
static int hf_mtp_transaction_id = -1;
static int hf_mtp_payload = -1;

extern bool mtp_parse_usb(const uint8_t *buffer, uint32_t len,
                          parsed_container *parsed);

#define ADD_STR_TO_TREE(field_name)                                            \
  proto_tree_add_string(mtp_tree, hf_mtp_##field_name, tvb,                    \
                        parsed.field_name##_start,                             \
                        parsed.field_name##_type_len, parsed.field_name);

#define ADD_INT_TO_TREE(field_name)                                            \
  proto_tree_add_uint(mtp_tree, hf_mtp_##field_name, tvb,                      \
                      parsed.field_name##_start, parsed.field_name##_len,      \
                      parsed.field_name);

static int dissect_mtp(tvbuff_t *tvb, packet_info *pinfo, proto_tree *tree,
                       void *data _U_) {
  const guint32 packet_len = tvb_captured_length(tvb);
  const guint8 *packet_buffer =
      (guint8 *)tvb_memdup(pinfo->pool, tvb, 0, packet_len);

  parsed_container parsed = {0};
  if (!mtp_parse_usb(packet_buffer, packet_len, &parsed)) {
    return (int)packet_len;
  }

  col_set_str(pinfo->cinfo, COL_PROTOCOL, PROTOCOL_NAME_SHORT);
  col_add_fstr(pinfo->cinfo, COL_INFO, "%s, %s", parsed.container_type,
               parsed.opcode);

  proto_item *ti = proto_tree_add_item(tree, proto_mtp, tvb, 0,
                                       (gint)parsed.container_len, ENC_NA);
  proto_item_set_text(ti, "%s, Type: %s, Opcode: %s, TID: 0x%x",
                      PROTOCOL_NAME_FULL, parsed.container_type, parsed.opcode,
                      parsed.transaction_id);

  proto_tree *mtp_tree = proto_item_add_subtree(ti, ett_mtp);

  PARSED_CONTAINER_FIELDS(ADD_STR_TO_TREE, ADD_INT_TO_TREE);

  gint payload_offset = parsed.transaction_id_start + parsed.transaction_id_len;
  gint payload_len = tvb_captured_length(tvb) - payload_offset;
  if (payload_len > 0) {
    proto_tree_add_item(mtp_tree, hf_mtp_payload, tvb, payload_offset,
                        payload_len, ENC_NA);
  }

  return (int)tvb_captured_length(tvb);
}

void proto_register_mtp(void) {
  proto_mtp = proto_register_protocol(PROTOCOL_NAME_FULL, PROTOCOL_NAME_SHORT,
                                      PROTOCOL_FILTER_NAME);

  static hf_register_info hf[] = {
      {&hf_mtp_container_len,
       {"Container Length", "mtp.len", FT_UINT32, BASE_DEC, NULL, 0x0, NULL,
        HFILL}},
      {&hf_mtp_container_type,
       {"Container Type", "mtp.type", FT_STRING, BASE_NONE, NULL, 0x0, NULL,
        HFILL}},
      {&hf_mtp_opcode,
       {"Opcode", "mtp.opcode", FT_STRING, BASE_NONE, NULL, 0x0, NULL, HFILL}},
      {&hf_mtp_transaction_id,
       {"Transaction ID", "mtp.trans_id", FT_UINT32, BASE_HEX, NULL, 0x0, NULL,
        HFILL}},
      {&hf_mtp_payload,
       {"Payload", "mtp.payload", FT_BYTES, BASE_NONE, NULL, 0x0, NULL, HFILL}},
  };

  proto_register_field_array(proto_mtp, hf, array_length(hf));
  proto_register_subtree_array(ett, array_length(ett));
}

void proto_reg_handoff_mtp(void) {
  static dissector_handle_t mtp_handle;

  mtp_handle = create_dissector_handle(dissect_mtp, proto_mtp);
  dissector_add_uint("usb.bulk", 6, mtp_handle);
  dissector_add_uint("usb.bulk", 0xFF, mtp_handle);
}

G_MODULE_EXPORT void plugin_register(void) {
  static proto_plugin plugin;
  plugin.register_protoinfo = proto_register_mtp;
  plugin.register_handoff = proto_reg_handoff_mtp;
  proto_register_plugin(&plugin);
}
