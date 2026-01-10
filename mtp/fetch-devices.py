import requests
from pathlib import Path
import os
from pycparser import parse_file, c_generator
from pycparser.c_ast import BinaryOp, ID, UnaryOp

DEVICES_LIST_URL = "https://sourceforge.net/p/libmtp/code/ci/master/tree/src/music-players.h?format=raw"
OUTPUT_DIR = Path(os.path.abspath(__file__)).parent / "generated"
DEVICES_H = OUTPUT_DIR / "devices.h"
DEVICES_RS = OUTPUT_DIR / "devices.rs"


class DeviceFlag:
	def __init__(self, variant):
		self.variant = variant
		self.negated = False

	def negate(self):
		self.negated = True

	def __repr__(self):
		neg = "!" if self.negated else ""
		return f"{neg}DeviceFlags::{self.variant}"


class UsbDeviceFlag:
	def __init__(self, variant):
		self.variant = variant
		self.negated = False

	def negate(self):
		self.negated = True

	def __repr__(self):
		neg = "!" if self.negated else ""
		return f"{neg}UsbDeviceFlags::{self.variant}"


class FlagSetConstant:
	def __init__(self, variant):
		self.variant = variant
		self.negated = False

	def negate(self):
		self.negated = True

	def __repr__(self):
		neg = "!" if self.negated else ""
		return f"{neg}UsbDeviceFlagSet::{self.variant}"


FLAG_MAPPINGS = {
	"DEVICE_FLAG_BROKEN_MTPGETOBJPROPLIST_ALL": DeviceFlag("BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL"),
	"DEVICE_FLAG_BROKEN_SET_SAMPLE_DIMENSIONS": DeviceFlag("BROKEN_SET_SAMPLE_DIMENSIONS"),
	"DEVICE_FLAG_BROKEN_MTPGETOBJPROPLIST": DeviceFlag("BROKEN_MTP_GET_OBJECT_PROP_LIST"),
	"DEVICE_FLAG_CANNOT_HANDLE_DATEMODIFIED": DeviceFlag("CANNOT_HANDLE_DATEMODIFIED"),
	"DEVICE_FLAG_NONE": DeviceFlag("empty()"),
	"DEVICE_FLAG_OGG_IS_UNKNOWN": DeviceFlag("OGG_IS_UNKNOWN"),
	"DEVICE_FLAG_PLAYLIST_SPL_V1": DeviceFlag("PLAYLIST_SPL_V1"),
	"DEVICE_FLAG_PLAYLIST_SPL_V2": DeviceFlag("PLAYLIST_SPL_V2"),
	"DEVICE_FLAG_UNIQUE_FILENAMES": DeviceFlag("UNIQUE_FILENAMES"),
	"DEVICE_FLAG_BROKEN_BATTERY_LEVEL": DeviceFlag("BROKEN_BATTERY_LEVEL"),
	"DEVICE_FLAG_LONG_TIMEOUT": DeviceFlag("LONG_TIMEOUT"),
	"DEVICE_FLAG_PROPLIST_OVERRIDES_OI": DeviceFlag("PROPLIST_OVERRIDES_OI"),
	"DEVICE_FLAG_SAMSUNG_OFFSET_BUG": DeviceFlag("SAMSUNG_OFFSET_BUG"),
	"DEVICE_FLAG_FLAC_IS_UNKNOWN": DeviceFlag("FLAC_IS_UNKNOWN"),
	"DEVICE_FLAG_ONLY_7BIT_FILENAMES": DeviceFlag("ONLY_7BIT_FILENAMES"),
	"DEVICE_FLAG_IRIVER_OGG_ALZHEIMER": DeviceFlag("IRIVER_OGG_ALZHEIMER"),
	"DEVICE_FLAG_BROKEN_SEND_OBJECT_PROPLIST": DeviceFlag("BROKEN_SEND_OBJECT_PROP_LIST"),
	"DEVICE_FLAG_BROKEN_SET_OBJECT_PROPLIST": DeviceFlag("BROKEN_SET_OBJECT_PROP_LIST"),
	"DEVICE_FLAG_SWITCH_MODE_BLACKBERRY": DeviceFlag("SWITCH_MODE_BLACKBERRY"),

	"DEVICE_FLAG_NO_RELEASE_INTERFACE": UsbDeviceFlag("NO_RELEASE_INTERFACE"),
	"DEVICE_FLAG_UNLOAD_DRIVER": UsbDeviceFlag("UNLOAD_DRIVER"),
	"DEVICE_FLAG_FORCE_RESET_ON_CLOSE": UsbDeviceFlag("FORCE_RESET_ON_CLOSE"),
	"DEVICE_FLAG_ALWAYS_PROBE_DESCRIPTOR": UsbDeviceFlag("ALWAYS_PROBE_DESCRIPTOR"),
	"DEVICE_FLAG_NO_ZERO_READS": UsbDeviceFlag("NO_ZERO_READS"),
	"DEVICE_FLAG_IGNORE_HEADER_ERRORS": UsbDeviceFlag("IGNORE_HEADER_ERRORS"),

	"DEVICE_FLAGS_ANDROID_BUGS": FlagSetConstant("ANDROID_BUGS"),
	"DEVICE_FLAGS_SONY_NWZ_BUGS": FlagSetConstant("SONY_NWZ_BUGS"),
	"DEVICE_FLAGS_ARICENT_BUGS": FlagSetConstant("ARICENT_BUGS"),
}


class FlagSet:
	def __init__(self):
		self.base = []
		self.usb = []
		self.additional_sets = []

	def append(self, flag):
		if isinstance(flag, DeviceFlag):
			self.base.append(flag)
		elif isinstance(flag, UsbDeviceFlag):
			self.usb.append(flag)
		else:
			self.additional_sets.append(flag)

	def collect(self, expr, negated=False):
		# Device flags can either be a single ID or a bitwise OR/unary of multiple IDs.
		# We need to convert the names to their rust counterparts.

		# If it's a single ID, we can just look it up in the mapping
		if isinstance(expr, ID):
			flag = FLAG_MAPPINGS[expr.name]
			if negated:
				flag.negate()

			self.append(flag)
			return

		if isinstance(expr, BinaryOp):
			self.collect(expr.left)
			self.collect(expr.right)
			return

		if isinstance(expr, UnaryOp) and expr.op == "~":
			self.collect(expr.expr, True)
			return

		raise ValueError("Unknown device flag type: " + str(expr))

	def __repr__(self):
		if len(self.base) == 0:
			base = "DeviceFlags::empty()"
		else:
			base = " | ".join(str(x) for x in self.base)

		if len(self.usb) == 0:
			usb = "UsbDeviceFlags::empty()"
		else:
			usb = " | ".join(str(x) for x in self.usb)

		flag_sets = (
			[f"UsbDeviceFlagSet {{ base: {base}, usb: {usb} }}"]
			+ [str(item) for item in self.additional_sets]
		)

		return " | ".join(flag_sets)


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

			flag_set = FlagSet()
			flag_set.collect(element.exprs[4])

			f.write(
				f"\tUsbDeviceDescriptor {{\n\t\tvendor: {vendor},\n\t\tvendor_id: {vendor_id},\n\t\tproduct: {product},\n\t\tproduct_id: {product_id},\n\t\tflags: {flag_set}\n\t}},\n")
		f.write("]")


if __name__ == "__main__":
	main()
