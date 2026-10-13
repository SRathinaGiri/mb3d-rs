// a window filled red through winit + softbuffer, as the program draws
// (a check of the platform's presentation, e.g. under Wine)
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct App {
    ctx: Option<softbuffer::Context<OwnedDisplayHandle>>,
    win: Option<(Rc<Window>, softbuffer::Surface<OwnedDisplayHandle, Rc<Window>>)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let w = Rc::new(el.create_window(Window::default_attributes().with_title("wintest")).unwrap());
        let ctx = softbuffer::Context::new(el.owned_display_handle()).unwrap();
        let s = softbuffer::Surface::new(&ctx, w.clone()).unwrap();
        w.request_redraw();
        self.ctx = Some(ctx);
        self.win = Some((w, s));
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, e: WindowEvent) {
        match e {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::RedrawRequested => {
                let (w, s) = self.win.as_mut().unwrap();
                let size = w.inner_size();
                eprintln!("redraw {}x{}", size.width, size.height);
                s.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap()).unwrap();
                let mut b = s.buffer_mut().unwrap();
                b.fill(0x00FF2020);
                b.present().unwrap();
            }
            _ => {}
        }
    }
}

fn main() {
    let el = EventLoop::new().unwrap();
    el.run_app(&mut App::default()).unwrap();
}
