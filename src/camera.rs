use std::borrow::Cow;
use std::ffi::CStr;
use std::mem;

use ::context::Context;
use ::abilities::Abilities;
use ::media::{Media, FileMedia};
use ::port::Port;
use ::storage::Storage;

use ::handle::prelude::*;

/// A structure representing a camera connected to the system.
pub struct Camera {
    camera: *mut ::gphoto2::Camera,
}

impl Drop for Camera {
    fn drop(&mut self) {
        unsafe {
            ::gphoto2::gp_camera_unref(self.camera);
        }
    }
}

impl Camera {
    /// Opens the first detected camera.
    pub fn autodetect(context: &mut Context) -> ::Result<Self> {
        let mut ptr = unsafe { mem::uninitialized() };

        try_unsafe!(::gphoto2::gp_camera_new(&mut ptr));

        let camera = Camera { camera: ptr };

        try_unsafe!(::gphoto2::gp_camera_init(camera.camera, context.as_mut_ptr()));

        Ok(camera)
    }

    /// Captures a preview.
    pub fn capture_preview(&mut self, context: &mut Context) -> ::Result<FileMedia> {
        let mut file_media = FileMedia::create_mem().unwrap();

        try_unsafe! {
            ::gphoto2::gp_camera_capture_preview(self.camera,
                                                 file_media.as_mut_ptr(),
                                                 context.as_mut_ptr())
        };

        Ok(file_media)
    }

    /// Captures an image.
    pub fn capture_image(&mut self, context: &mut Context) -> ::Result<CameraFile> {
        let mut file_path = unsafe { mem::uninitialized() };

        try_unsafe! {
            ::gphoto2::gp_camera_capture(self.camera,
                                         ::gphoto2::GP_CAPTURE_IMAGE,
                                         &mut file_path,
                                         context.as_mut_ptr())
        };

        Ok(CameraFile { inner: file_path })
    }

    /// Downloads a file from the camera.
    pub fn download<T: Media>(&mut self, context: &mut Context, source: &CameraFile, destination: &mut T) -> ::Result<()> {
        try_unsafe! {
            ::gphoto2::gp_camera_file_get(self.camera,
                                          source.inner.folder.as_ptr(),
                                          source.inner.name.as_ptr(),
                                          ::gphoto2::GP_FILE_TYPE_NORMAL,
                                          destination.as_mut_ptr(),
                                          context.as_mut_ptr())
        };

        Ok(())
    }

    /// Returns information about the port the camera is connected to.
    pub fn port<'a>(&'a self) -> Port<'a> {
        let mut ptr = unsafe { mem::uninitialized() };

        unsafe {
            assert_eq!(::gphoto2::GP_OK, ::gphoto2::gp_camera_get_port_info(self.camera, &mut ptr));
        }

        ::port::from_libgphoto2(self, ptr)
    }

    /// Retrieves the camera's abilities.
    pub fn abilities(&self) -> Abilities {
        let mut abilities = unsafe { mem::uninitialized() };

        unsafe {
            assert_eq!(::gphoto2::GP_OK, ::gphoto2::gp_camera_get_abilities(self.camera, &mut abilities));
        }

        ::abilities::from_libgphoto2(abilities)
    }

    /// Retrieves information about the camera's storage.
    ///
    /// Returns a `Vec` containing one `Storage` for each filesystem on the device.
    pub fn storage(&mut self, context: &mut Context) -> ::Result<Vec<Storage>> {
        let mut ptr = unsafe { mem::uninitialized() };
        let mut len = unsafe { mem::uninitialized() };

        try_unsafe! {
            ::gphoto2::gp_camera_get_storageinfo(self.camera,
                                                 &mut ptr,
                                                 &mut len,
                                                 context.as_mut_ptr())
        };

        let storage = ptr as *mut Storage;
        let length = len as usize;

        Ok(unsafe { Vec::from_raw_parts(storage, length, length) })
    }

    /// Returns the camera's summary.
    ///
    /// The summary typically contains non-configurable information about the camera, such as
    /// manufacturer and number of pictures taken.
    ///
    /// ## Errors
    ///
    /// This function returns an error if the summary could not be retrieved:
    ///
    /// * `NotSupported` if there is no summary available for the camera.
    /// * `CorruptedData` if the summary is invalid UTF-8.
    pub fn summary(&mut self, context: &mut Context) -> ::Result<String> {
        let mut summary = unsafe { mem::uninitialized() };

        try_unsafe!(::gphoto2::gp_camera_get_summary(self.camera, &mut summary, context.as_mut_ptr()));

        util::camera_text_to_string(summary)
    }

    /// Returns the camera's manual.
    ///
    /// The manual contains information about using the camera.
    ///
    /// ## Errors
    ///
    /// This function returns an error if the manual could not be retrieved:
    ///
    /// * `NotSupported` if there is no manual available for the camera.
    /// * `CorruptedData` if the summary is invalid UTF-8.
    pub fn manual(&mut self, context: &mut Context) -> ::Result<String> {
        let mut manual = unsafe { mem::uninitialized() };

        try_unsafe!(::gphoto2::gp_camera_get_manual(self.camera, &mut manual, context.as_mut_ptr()));

        util::camera_text_to_string(manual)
    }

    /// Returns information about the camera driver.
    ///
    /// This text typically contains information about the driver's author, acknowledgements, etc.
    ///
    /// ## Errors
    ///
    /// This function returns an error if the about text could not be retrieved:
    ///
    /// * `NotSupported` if there is no about text available for the camera's driver.
    /// * `CorruptedData` if the summary is invalid UTF-8.
    pub fn about_driver(&mut self, context: &mut Context) -> ::Result<String> {
        let mut about = unsafe { mem::uninitialized() };

        try_unsafe!(::gphoto2::gp_camera_get_about(self.camera, &mut about, context.as_mut_ptr()));

        util::camera_text_to_string(about)
    }
}


/// A file stored on a camera's storage.
pub struct CameraFile {
    inner: ::gphoto2::CameraFilePath,
}

impl CameraFile {
    /// Returns the directory that the file is stored in.
    pub fn directory(&self) -> Cow<str> {
        unsafe {
            String::from_utf8_lossy(CStr::from_ptr(self.inner.folder.as_ptr()).to_bytes())
        }
    }

    /// Returns the name of the file without the directory.
    pub fn basename(&self) -> Cow<str> {
        unsafe {
            String::from_utf8_lossy(CStr::from_ptr(self.inner.name.as_ptr()).to_bytes())
        }
    }
}

mod util {
    use std::ffi::CStr;

    pub fn camera_text_to_string(mut camera_text: ::gphoto2::CameraText) -> ::Result<String> {
        let length = unsafe {
            CStr::from_ptr(camera_text.text.as_ptr()).to_bytes().len()
        };

        let vec = unsafe {
            Vec::<u8>::from_raw_parts(camera_text.text.as_mut_ptr() as *mut u8, length, camera_text.text.len())
        };

        String::from_utf8(vec).map_err(|_| {
            ::error::from_libgphoto2(::gphoto2::GP_ERROR_CORRUPTED_DATA)
        })
    }
}
