macro_rules! cookbook_directory_device {
    ($m:ident) => {
        $m!(
            "device/reference",
            "Reference Device",
            "device-reference",
            Some(crate::runtime::reference_device::RECIPES),
            || Box::new(crate::runtime::reference_device::ReferenceDeviceLib)
        );
        $m!(
            "stream-device",
            "Stream Device",
            "device-reference",
            Some(crate::lib_stream_device::RECIPES),
            || Box::new(crate::lib_stream_device::DeviceStreamBaseLib)
        );
    };
}
