//! Contains the implementation of the Mac OS X tray icon in the top bar.

use std::{self, sync::mpsc::Sender, thread};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

use cocoa::{
	appkit::{NSApp, NSApplication, NSButton, NSImage, NSSquareStatusItemLength, NSStatusBar,
			 NSStatusItem},
	base::{id, nil},
	foundation::{NSAutoreleasePool, NSData, NSSize},
};
use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use objc::runtime::Object;

use crate::{Error, SystrayEvent};

// safily move object between threads
#[derive(Clone, Debug)]
pub struct SafeId(Arc<Mutex<*mut Object>>);

impl SafeId {
	fn new(id: id) -> Self {
		SafeId(Arc::new(Mutex::new(id)))
	}
}

unsafe impl Send for SafeId {}

unsafe impl Sync for SafeId {}


/// event for comunicated with the app running on the main thread
pub enum OsxSystemTrayEvent {
	/// don't allocate for the image buffer
	ChangeImage(&'static [u8]),
	Shutdown,
}

const ICON_WIDTH: f64 = 32.0;
const ICON_HEIGHT: f64 = 32.0;

/// The generation representation of the Mac OS X application.
pub struct Window {
	/// A mutable reference to the `NSApplication` instance of the currently running application.
	application: SafeId,
	/// It seems that we have to use `NSAutoreleasePool` to prevent memory leaks.
	autorelease_pool: SafeId,
	tray: SafeId,
	/// sender for the wrapper
	event_tx: Sender<SystrayEvent>,
	/// the handler will be the sender for the running app which is consuming the main thread
	handler: Sender<OsxSystemTrayEvent>,
}

impl Window {
	/// Creates a new instance of the `Window`.
	pub fn new(event_tx: Sender<SystrayEvent>) -> Result<Window, Error> {
		let (handler_tx, handler_rx) = channel();
		let mut app = unsafe { NSApp() };
		unsafe { app.setActivationPolicy_(NSApplicationActivationPolicyRegular); }
		let bool = unsafe { NSAutoreleasePool::new(nil) };
		let mut window = Window {
			application: unsafe { SafeId::new(NSApp()) },
			autorelease_pool: SafeId::new(bool),
			tray: SafeId::new(Window::init_tray()),
			event_tx,
			handler: handler_tx,
		};
		let lister_thread = window.run_lister(handler_rx);
		Ok(window)
	}
	fn init_tray() -> id {
		unsafe {
			NSStatusBar::systemStatusBar(nil)
				.statusItemWithLength_(NSSquareStatusItemLength)
				.autorelease()
		}
	}
	pub(crate) fn run(&mut self) {
		unsafe {
			self.application.clone().0.lock().unwrap().run();
		}
	}
	fn run_lister(&mut self, rx: Receiver<OsxSystemTrayEvent>) -> JoinHandle<()> {
		let tray = self.tray.clone();
		thread::spawn(move || loop {
			let lister = rx.try_iter();
			for e in lister {
				match e {
					OsxSystemTrayEvent::ChangeImage(image) => unsafe {
						let nsdata = NSData::dataWithBytes_length_(
							nil,
							image.as_ptr() as *const std::os::raw::c_void,
							image.len() as u64,
						)
							.autorelease();

						let nsimage = unsafe {
							NSImage::initWithData_(NSImage::alloc(nil), nsdata).autorelease()
						};
						let new_size = NSSize::new(ICON_WIDTH, ICON_HEIGHT);

						let r: () = msg_send![nsimage, setSize: new_size];
						tray.0.lock().unwrap().button().setImage_(nsimage);
					},
					OsxSystemTrayEvent::Shutdown => {
						unimplemented!();
					}
				}
			}
		})
	}
	/// Closes the current application.
	pub fn quit(&self) {
		// let app = self.application.0.clone().lock().unwrap();
		// let _: () = unsafe { msg_send![app, terminate] };
		unimplemented!()
	}

	/// Sets the tooltip (not available for this platfor).
	pub fn set_tooltip(&self, _: &str) -> Result<(), Error> {
		Err(Error::OsError("This operating system does not support tooltips for the tray \
                                   items".to_owned()))
	}

	/// Adds an additional item to the tray icon menu.
	pub fn add_menu_item<F>(&self, _: &String, _: F) -> Result<u32, Error>
		where F: std::ops::Fn(&Window) -> () + 'static
	{
		unimplemented!()
	}

	/// Sets the application icon displayed in the tray bar. Accepts a `buffer` to the underlying
	/// image, you can pass even encoded PNG images here. Supports the same list of formats as
	/// `NSImage`.
	pub fn set_icon_from_buffer(&mut self, buffer: &'static [u8], _: u32, _: u32)
								-> Result<(), Error> {
		dbg!(buffer);
		self.handler.send(OsxSystemTrayEvent::ChangeImage(buffer)).unwrap();
		Ok(())
	}

	/// Starts the application event loop. Calling this function will block the current thread.
	pub fn wait_for_message(&mut self) -> Result<(),()>{
		thread::spawn(move || {
			loop {
				log::debug!("listinging ..");
				thread::sleep(Duration::from_millis(100));
			}
		});

		Ok(())
	}

	pub fn set_icon_from_resource(&self, resource_name: &str) -> Result<(), Error> {
		unimplemented!()
	}

	pub fn set_icon_from_file(&self, icon_file: &str) -> Result<(), Error> {
		unimplemented!()
	}

	pub fn add_menu_separator(&self, item_idx: u32) -> Result<(), Error> {
		unimplemented!()
	}

	pub fn add_menu_entry(&self, item_idx: u32, item_name: &str) -> Result<(), Error> {
		unimplemented!()
	}

	pub fn shutdown(&self) -> Result<(), Error> {
		Ok(())
	}
}
