import requests
from pathlib import Path
import os
from pycparser import parse_file, c_generator
from pycparser.c_ast import BinaryOp, ID, UnaryOp

DEVICES_LIST_URL = "https://sourceforge.net/p/libmtp/code/ci/master/tree/src/music-players.h?format=raw"
OUTPUT_DIR = Path(os.path.abspath(__file__)).parent / "generated"
DEVICES_H = OUTPUT_DIR / "devices.h"
DEVICES_RS = OUTPUT_DIR / "devices.rs"


def main():
	if not OUTPUT_DIR.exists():
		OUTPUT_DIR.mkdir()

	fetch()

	# Format:
	# { char* vendor, uint16_t vendor_id, char* product, uint16_t product_id, uint32_t device_flags }
	ast = parse_file(DEVICES_H, use_cpp=True)

	# Get the array
	decl = ast.ext[0]
	assert decl.name == "mtp_device_table"

	# Get the array elements
	elements = decl.init.exprs
	assert len(elements) > 0

	with open(DEVICES_RS, "w+") as f:
		f.write("[\n")
		for element in elements:
			vendor = element.exprs[0].value
			vendor_id = element.exprs[1].value
			product = element.exprs[2].value
			product_id = element.exprs[3].value
			device_flags = rewrite_device_flags(element.exprs[4])

			f.write(
				f"\tUsbDeviceDescriptor {{\n\t\tvendor: {vendor},\n\t\tvendor_id: {vendor_id},\n\t\tproduct: {product},\n\t\tproduct_id: {product_id},\n\t\tflags: {device_flags}\n\t}},\n")
		f.write("]")


DEVICE_FLAG_MAPPINGS = {
	"DEVICE_FLAG_BROKEN_MTPGETOBJPROPLIST_ALL": "UsbDeviceFlags::BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL",
	"DEVICE_FLAG_NO_RELEASE_INTERFACE": "UsbDeviceFlags::NO_RELEASE_INTERFACE",
	"DEVICE_FLAG_IGNORE_HEADER_ERRORS": "UsbDeviceFlags::IGNORE_HEADER_ERRORS",
	"DEVICE_FLAGS_ANDROID_BUGS": "UsbDeviceFlags::ANDROID_BUGS",
	"DEVICE_FLAG_BROKEN_SET_SAMPLE_DIMENSIONS": "UsbDeviceFlags::BROKEN_SET_SAMPLE_DIMENSIONS",
	"DEVICE_FLAG_UNLOAD_DRIVER": "UsbDeviceFlags::UNLOAD_DRIVER",
	"DEVICE_FLAG_BROKEN_MTPGETOBJPROPLIST": "UsbDeviceFlags::BROKEN_MTP_GET_OBJECT_PROP_LIST",
	"DEVICE_FLAG_ALWAYS_PROBE_DESCRIPTOR": "UsbDeviceFlags::ALWAYS_PROBE_DESCRIPTOR",
	"DEVICE_FLAG_CANNOT_HANDLE_DATEMODIFIED": "UsbDeviceFlags::CANNOT_HANDLE_DATEMODIFIED",
	"DEVICE_FLAG_NONE": "UsbDeviceFlags::empty()",
	"DEVICE_FLAG_OGG_IS_UNKNOWN": "UsbDeviceFlags::OGG_IS_UNKNOWN",
	"DEVICE_FLAG_PLAYLIST_SPL_V1": "UsbDeviceFlags::PLAYLIST_SPL_V1",
	"DEVICE_FLAG_NO_ZERO_READS": "UsbDeviceFlags::NO_ZERO_READS",
	"DEVICE_FLAG_PLAYLIST_SPL_V2": "UsbDeviceFlags::PLAYLIST_SPL_V2",
	"DEVICE_FLAG_UNIQUE_FILENAMES": "UsbDeviceFlags::UNIQUE_FILENAMES",
	"DEVICE_FLAG_BROKEN_BATTERY_LEVEL": "UsbDeviceFlags::BROKEN_BATTERY_LEVEL",
	"DEVICE_FLAG_LONG_TIMEOUT": "UsbDeviceFlags::LONG_TIMEOUT",
	"DEVICE_FLAG_PROPLIST_OVERRIDES_OI": "UsbDeviceFlags::PROPLIST_OVERRIDES_OI",
	"DEVICE_FLAG_SAMSUNG_OFFSET_BUG": "UsbDeviceFlags::SAMSUNG_OFFSET_BUG",
	"DEVICE_FLAG_FLAC_IS_UNKNOWN": "UsbDeviceFlags::FLAC_IS_UNKNOWN",
	"DEVICE_FLAG_ONLY_7BIT_FILENAMES": "UsbDeviceFlags::ONLY_7BIT_FILENAMES",
	"DEVICE_FLAG_IRIVER_OGG_ALZHEIMER": "UsbDeviceFlags::IRIVER_OGG_ALZHEIMER",
	"DEVICE_FLAG_BROKEN_SEND_OBJECT_PROPLIST": "UsbDeviceFlags::BROKEN_SEND_OBJECT_PROP_LIST",
	"DEVICE_FLAGS_SONY_NWZ_BUGS": "UsbDeviceFlags::SONY_NWZ_BUGS",
	"DEVICE_FLAG_BROKEN_SET_OBJECT_PROPLIST": "UsbDeviceFlags::BROKEN_SET_OBJECT_PROP_LIST",
	"DEVICE_FLAG_SWITCH_MODE_BLACKBERRY": "UsbDeviceFlags::SWITCH_MODE_BLACKBERRY",
	"DEVICE_FLAG_FORCE_RESET_ON_CLOSE": "UsbDeviceFlags::FORCE_RESET_ON_CLOSE",
}


def rewrite_device_flags(expr):
	# Device flags can either be a single ID or a bitwise OR/unary of multiple IDs.
	# We need to convert the names to their rust counterparts.

	# If it's a single ID, we can just look it up in the mapping
	if isinstance(expr, ID):
		return DEVICE_FLAG_MAPPINGS[expr.name]

	# If it's a bitwise OR, we need to recurse
	if isinstance(expr, BinaryOp):
		left = rewrite_device_flags(expr.left)
		right = rewrite_device_flags(expr.right)
		return f"{left} | {right}"

	# If it's a unary, we need to recurse
	if isinstance(expr, UnaryOp) and expr.op == "~":
		# Bitwise NOT in rust is !, not ~
		return "!" + rewrite_device_flags(expr.expr)

	raise ValueError("Unknown device flag type: " + str(expr))


def fetch():
	if DEVICES_H.exists():
		print("devices.h already exists, skipping fetch")
		return

	response = requests.get(DEVICES_LIST_URL)
	response.raise_for_status()

	with open(DEVICES_H, "x") as f:
		# We have to inject the array declaration to get it to parse
		text = "{\n" + response.text + "\n};"
		f.write(text)


if __name__ == "__main__":
	main()
