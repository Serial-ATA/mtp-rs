# MTPFS

This is a [FUSE] filesystem for MTP-compatible devices. It allows you to browse the files the device's storage as if it were an external drive.

## Limitations

Using this comes with many drawbacks, due to the design of MTP.

### Initial Permissions Check

This will ***most likely*** not work on the first run. The device will be notified that media access is requested, which
may lead to a confirmation prompt on the device, and an error from `MTPFS`. In that case, accepting the prompt and rerunning should
fix the issue.

### Speed

Due to MTP's design, the initial read ***will*** be slow, and you may notice delay when opening or modifying files.

When initially loading a storage, we receive the folders and files one-by-one, and in arbitrary order. This means we must wait until
every entry is received before we can start building the tree. The limiting factors are the number of files and folders on the device,
and the speed of the device.

When reading or modifying files, this too depends on the speed of the device, as contents are fetched on-demand.

For known slow operations, there will be a spinner shown in the CLI, to indicate that work *is* being done.

### Limited Modifications

Devices may restrict certain write operations, such as changing the name of a file/folder. This depends entirely on the device, and
cannot be worked around.

### Limited Information

Some devices may not allow you to fetch certain properties, such as modification date or file size. As with the above, there is no way to
work around this, so the information presented may not be entirely accurate.

[FUSE]: https://docs.kernel.org/filesystems/fuse.html
