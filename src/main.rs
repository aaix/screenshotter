#![windows_subsystem = "windows"]

use std::{error::Error};

use rand::Rng;
use windows::{Win32::{UI::{Input::KeyboardAndMouse::{VK_SNAPSHOT, VIRTUAL_KEY, self, VK_F10}, WindowsAndMessaging::*}, Graphics::{Direct3D11::*, Dxgi::{*, Common::*}, Direct3D::*}, Foundation::{self, GetLastError}, System::LibraryLoader::LoadLibraryA}, core::ComInterface, s};

macro_rules! debug {
    ($($t:tt)*) => {{
        win_dbg_logger::output_debug_string(&(format!($($t)*) + "\n"));
    }};
}

pub const D3D11_CPU_ACCESS_NONE: D3D11_CPU_ACCESS_FLAG = D3D11_CPU_ACCESS_FLAG(0i32);

fn main() {
    // output panic message to debug stream
    std::panic::set_hook(Box::new(|p| {
        debug!("{p}");
    }));
    debug!("Hello, world!");

    #[cfg(debug_assertions)]
    unsafe {LoadLibraryA(s!(r"C:\Program Files\Microsoft PIX\2305.10\WinPixGpuCapturer.dll")).unwrap();}


    register_hotey(VK_F10);

    let mut state = DXGIState::new().unwrap();


    debug!("output desc is {:?}", state.get_output_desc());
    debug!("Output dimensions are {:?}", state.get_output_desc().DesktopCoordinates.dimensions());

    loop {
        let mut msg = MSG::default();

        if unsafe {GetMessageA(&mut msg as *mut _, None, 0, 0)}.as_bool() {
            // There is a message available
            match msg.message {
                WM_HOTKEY => {
                    state.capture_screen().unwrap();
                    state.show_window();
                },

                WM_PAINT => {
                    if state.has_frame {
                        state.paint_frame();
                    }
                },

                WM_QUIT | WM_CLOSE => {
                    state.hide_window();
                }

                WM_TIMER => {}

                _ => {debug!("Unknown message : {:#X}", msg.message); state.has_frame = true;}
            }
        }

    }
    

    
}

fn register_hotey(key: VIRTUAL_KEY) {
    unsafe {
        KeyboardAndMouse::RegisterHotKey(
            None,
            0,
            KeyboardAndMouse::MOD_NOREPEAT,
            key.0 as u32
        ).ok().unwrap()
    };
}

struct DXGIState {
    factory: IDXGIFactory7,
    device: ID3D11Device5,
    device_context: ID3D11DeviceContext4,
    adapter: IDXGIAdapter4,
    output: IDXGIOutput6,
    window: Foundation::HWND,
    swapchain: IDXGISwapChain4,
    screenshot: Option<ID3D11Texture2D1>,

    has_frame: bool,
}

impl DXGIState {
    fn new() -> Result<Self, Box<dyn Error>> {

        unsafe { windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE 
        )};


        let factory: IDXGIFactory7 = unsafe {
            #[cfg(debug_assertions)]
            let flags = DXGI_CREATE_FACTORY_DEBUG;
            #[cfg(not(debug_assertions))]
            let flags = 0;

            CreateDXGIFactory2::<IDXGIFactory7>(flags)?
        };

        let adapter: IDXGIAdapter4 = unsafe {factory.EnumAdapters1(0)?.cast::<IDXGIAdapter4>()} ?;

        let output: IDXGIOutput6 = unsafe {adapter.EnumOutputs(0)?.cast::<IDXGIOutput6>()}?;

        let dimensions = unsafe {
            let mut desc: DXGI_OUTPUT_DESC = DXGI_OUTPUT_DESC::default();
            output.GetDesc(&mut desc as *mut _)?;

            desc.DesktopCoordinates.dimensions()

            
        };

        let window: Foundation::HWND = unsafe {
            let handle: Foundation::HWND = CreateWindowExA(
                WS_EX_LEFT,
                s!("Static"),
                s!("screencapture select area"),
                WS_POPUP,
                dimensions.x,
                dimensions.y,
                dimensions.width as i32,
                dimensions.height as i32,
                None,
                None,
                None,
                None
            );

            if handle.0 == 0 {
                GetLastError().ok()?;
                // if handle is 0 then there should be an error.
                unreachable!();
            };
            handle
        };

        let (device, device_context, swapchain) = unsafe {

            #[cfg(not(debug_assertions))]
            let flags = D3D11_CREATE_DEVICE_SINGLETHREADED;
            
            #[cfg(debug_assertions)]
            let flags: D3D11_CREATE_DEVICE_FLAG = D3D11_CREATE_DEVICE_DEBUG;

            let swapchain_desc = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC {
                    Width: dimensions.width,
                    Height: dimensions.height,
                    RefreshRate: DXGI_RATIONAL {
                        Numerator: 60,
                        Denominator: 1
                    },
                    Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
                    ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_PROGRESSIVE,
                    Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                },
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0
                },
                BufferUsage: DXGI_USAGE_BACK_BUFFER,
                BufferCount: 2,
                OutputWindow: window,
                Windowed: true.into(),
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                Flags: 0,
            };


            let mut swapchain: Option<IDXGISwapChain> = None;
            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;

            D3D11CreateDeviceAndSwapChain(
            &adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            None,
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(&swapchain_desc as *const _),
            Some(&mut swapchain as *mut _),
            Some(&mut device as *mut _),
            None,
            Some(&mut context as *mut _)
            )?;
            (
                device.unwrap().cast::<ID3D11Device5>()?,
                context.unwrap().cast::<ID3D11DeviceContext4>()?,
                swapchain.unwrap().cast::<IDXGISwapChain4>()?
            )
        };
        


        Ok(Self {
            factory,
            device,
            device_context,
            adapter,
            output,
            window,
            swapchain,
            screenshot: None,
            has_frame: true,
        })
    }

    fn get_output_desc(&self) -> DXGI_OUTPUT_DESC1 {
        unsafe {
            let mut desc: DXGI_OUTPUT_DESC1 = DXGI_OUTPUT_DESC1::default();
            self.output.GetDesc1(&mut desc as *mut _).unwrap();
            desc
        }
    }

    fn show_window(&self) -> bool {
        unsafe {ShowWindow(self.window, SW_SHOW)}.as_bool()
    }

    fn hide_window(&self) -> bool {
        unsafe {ShowWindow(self.window, SW_HIDE)}.as_bool()

    }

    fn capture_screen(&mut self) -> Result<(), Box<dyn Error>> {
        let ctx: IDXGIOutputDuplication = unsafe {
            self.output.DuplicateOutput1(
            &self.device,
                0,
                &[
                    DXGI_FORMAT_R16G16B16A16_FLOAT,
                ]
            )?
        };

        let resource: ID3D11Texture2D1 = Self::create_texture(
            &self.device,
            &self.get_output_desc().DesktopCoordinates.dimensions(),
            D3D11_USAGE_DEFAULT,
            D3D11_CPU_ACCESS_NONE,
        )?;

        let frame_info: DXGI_OUTDUPL_FRAME_INFO = unsafe {
            let mut frame_info: DXGI_OUTDUPL_FRAME_INFO = DXGI_OUTDUPL_FRAME_INFO::default();
            ctx.AcquireNextFrame(
                0,
                &mut frame_info as *mut _,
                &mut Some(resource.cast::<IDXGIResource>()?) as *mut _
            )?;
            frame_info
        };

        debug!("{:?}", frame_info);
        unsafe {
            let mut desc = D3D11_TEXTURE2D_DESC1::default();
            resource.GetDesc1(&mut desc as *mut _);
            debug!("{:?}", desc);
        };

        let texture: ID3D11Texture2D1 = Self::create_texture(
            &self.device,
            &self.get_output_desc().DesktopCoordinates.dimensions(),
            D3D11_USAGE_DEFAULT,
            D3D11_CPU_ACCESS_NONE,
        )?;

        unsafe {self.device_context.CopyResource(&texture, &resource)}

        self.screenshot = Some(texture);


        Ok(())
    }

    fn paint_frame(&mut self) {
        if self.screenshot.is_none() {
            return
        };

        let frame = Self::create_texture(
            &self.device,
            &self.get_output_desc().DesktopCoordinates.dimensions(),
            D3D11_USAGE_DEFAULT,
            D3D11_CPU_ACCESS_WRITE
        ).unwrap();

        let resource = self.screenshot.as_ref().unwrap();

        //unsafe {self.device_context.CopyResource(&frame,resource)};


        unsafe {
            let mut data: Vec<u128> = vec![1; 3840 * 2160 * 4 /8];
            let rng = rand::thread_rng().gen::<u128>();
            data.fill(rng);

            self.device_context.UpdateSubresource(&frame, 0, None, data.as_ptr() as *const _, 3840 * 4, 3840 * 2160 * 4)



        }
        


        //TODO: modify frame here

        let buffer = unsafe {self.swapchain.GetBuffer::<ID3D11Texture2D>(0).unwrap()};

        unsafe {self.device_context.CopyResource(&buffer, &frame)}

        // TODO: use mark dirty rects with present1
        match unsafe {self.swapchain.Present(1, 0)}.ok() {
            Ok(()) => {},
            Err(e) => {debug!("Error presenting {:?}", e)}
        };

        self.has_frame = false;
        
    }

    fn create_texture(
        device: &ID3D11Device5,
        dimensions: &Dimensions,
        usage: D3D11_USAGE,
        cpu_access: D3D11_CPU_ACCESS_FLAG
    ) -> Result<ID3D11Texture2D1, Box<dyn Error>> {
        Ok(unsafe {
            let mut texture: Option<ID3D11Texture2D1> = None;
            device.CreateTexture2D1(
                &D3D11_TEXTURE2D_DESC1 {
                    Width: dimensions.width,
                    Height: dimensions.height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0
                    },
                    Usage: usage,
                    BindFlags: D3D11_BIND_FLAG(0),
                    CPUAccessFlags: cpu_access,
                    MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                    TextureLayout: {
                        if cpu_access.0 != 0 {
                            D3D11_TEXTURE_LAYOUT_ROW_MAJOR
                        } else {
                            D3D11_TEXTURE_LAYOUT_UNDEFINED
                        }
                    },
                } as *const _,
                None,
                Some(&mut texture as *mut _),

            )?;
            texture.unwrap()
        })
    }
}

trait HasDimensions {
    fn dimensions(&self) -> Dimensions;
}
#[derive(Debug)]
struct Dimensions {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

impl HasDimensions for Foundation::RECT {
    fn dimensions(&self) -> Dimensions {
        Dimensions {
            width: (self.right-self.left).try_into().unwrap(),
            height: (self.bottom-self.top).try_into().unwrap(),
            x: self.left,
            y: self.top
        }
    }
}