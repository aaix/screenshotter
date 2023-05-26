#![windows_subsystem = "windows"]

use std::error::Error;
use windows::{
    Win32::{
        UI::{
            Input::KeyboardAndMouse::{
                VK_SNAPSHOT,
                VIRTUAL_KEY,
                self, VK_ESCAPE
            },
            WindowsAndMessaging::*
        },
        Graphics::{
            Direct3D11::*,
            Dxgi::{
                *,
                Common::*
            },
            Direct3D::*
        }, Foundation::{
            self,
            GetLastError
        },
        System::{
            LibraryLoader::LoadLibraryA,
            Diagnostics::Debug::OutputDebugStringW
        },
    },
    core::{
        ComInterface,
        PCWSTR, PCSTR
    },
    s, w
};

macro_rules! debug {
    ($($t:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe {
            OutputDebugStringW(
                PCWSTR::from_raw(
                    (&(format!($($t)*) + "\n\0").encode_utf16().collect::<Vec<u16>>()[0] as *const u16))
            );
        }
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
    {
        if let Err(e) = unsafe {LoadLibraryA(s!(r"C:\Program Files\Microsoft PIX\2305.10\WinPixGpuCapturer.dll"))} {
            debug!("Couldnt load PIX library {:?}", e)
        };
    };

    register_hotey(VK_SNAPSHOT);

    let mut state = DXGIState::new().unwrap();

    debug!("Output dimensions are {:?}", state.get_output_desc().DesktopCoordinates.dimensions());

    let mut msg = MSG::default();
    loop {
        
        if unsafe {GetMessageA(&mut msg as *mut _, None, 0, 0)}.as_bool() {
            // There is a message available
            match msg.message {
                WM_HOTKEY => {
                    state.capture_screen().unwrap();
                    state.show_window();
                    state.paint_frame();
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

                WM_KEYDOWN | WM_KEYUP | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE => {
                    state.process_input(msg);
                }

                // redraw occasionally
                _ => {state.has_frame = true;}
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

#[derive(Debug)]
struct InputState {
    corner1: (i32, i32),
    corner2: Option<(i32, i32)>
}

impl HasDimensions for InputState {
    fn dimensions(&self) -> Dimensions {
        let c1 = self.corner1;
        if let Some(c2) = self.corner2 {

            // c1 is left
            if c1.0 < c2.0 {
                //c1 is top
                if c1.1 < c2.1 {
                    Dimensions {width: (c2.0 - c1.0).try_into().unwrap(), height: (c2.1 - c1.1).try_into().unwrap(), x: c1.0, y: c1.1}
                } // c2 is top
                else {
                    let width = (c2.0 - c1.0).try_into().unwrap();
                    let height = (c1.1 - c2.1).try_into().unwrap();

                    Dimensions {width, height, x: c2.0 - width as i32, y: c1.1 - height as i32}
                }
            } // c2 is left
            else if c1.0 > c2.0 {
                //c1 is top
                if c1.1 < c2.1 {
                    let width = (c1.0 - c2.0).try_into().unwrap();
                    let height = (c2.1 - c1.1).try_into().unwrap();
                    Dimensions {width, height, x: c1.0 - width as i32, y: c2.1 - height as i32}

                } // c2 is top
                else {
                    Dimensions {width: (c1.0 - c2.0).try_into().unwrap(), height: (c1.1 - c2.1).try_into().unwrap(), x: c2.0, y: c2.1}
                }
            } // c1==c2
            else {
                Dimensions {width: 0, height: 0, x: c1.0, y: c1.1}
            }
        } else {
            Dimensions {width: 0, height: 0, x: c1.0, y: c1.1}
        }
    }
}
struct DXGIState {
    // graphics objects
    factory: IDXGIFactory7,
    device: ID3D11Device5,
    device_context: ID3D11DeviceContext4,
    adapter: IDXGIAdapter4,
    output: IDXGIOutput6,
    window: Foundation::HWND,
    swapchain: IDXGISwapChain4,
    render_target: ID3D11Texture2D1,

    // processing state
    screenshot: Option<ID3D11Texture2D1>,
    has_frame: bool,
    input_state: Option<InputState>,
    state_resource: ID3D11Buffer,
    use_dirty_rects: bool,
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
            let flags = 0;
            
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

        // shaders
        let vertex_shader_blob = unsafe {Self::create_shader(
            w!("Shaders.hlsl"),
            s!("VS_main"),
            s!("vs_5_0")
        )}?;

        let pixel_shader_blob = unsafe {Self::create_shader(
            w!("Shaders.hlsl"),
            s!("PS_main"),
            s!("ps_5_0")
        )}?;

        let (vertex_shader, pixel_shader) =unsafe {
            let mut ppvertexshader: Option<ID3D11VertexShader> = None;
            let mut pppixelshader: Option<ID3D11PixelShader> = None;
            device.CreateVertexShader(
                blob_as_slice(&vertex_shader_blob),
                None,
                Some(&mut ppvertexshader as *mut _)
            )?;
            device.CreatePixelShader(
                blob_as_slice(&pixel_shader_blob),
                None,
                Some(&mut pppixelshader as *mut _)
            )?;
            (ppvertexshader.unwrap(), pppixelshader.unwrap())
        };


        // renderer
        let vertices: [Vertex; 4] = [
            Vertex ([-1.0,  1.0], [0.0, 0.0]),
            Vertex ([1.0, 1.0], [1.0, 0.0]),
            Vertex ([-1.0 , -1.0], [0.0, 1.0]),
            Vertex ([1.0, -1.0], [1.0, 1.0])
        ];


        let vertex_buffer = unsafe {
            let mut buffer: Option<ID3D11Buffer> = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of_val(&vertices) as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_VERTEX_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_NONE,
                    MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                    StructureByteStride: 0,
                } as *const _,
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: (&vertices as *const _) as *const _,
                    SysMemPitch: std::mem::size_of_val(&vertices) as u32,
                    SysMemSlicePitch: 0,
                } as *const _),
                Some(&mut buffer as *mut _)
            )?;
            buffer.unwrap()
        };

        let input_layout = unsafe {
            let mut layout: Option<ID3D11InputLayout> = None;
            device.CreateInputLayout(
                &[
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("POSITION"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: 0,
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("TEXCOORD"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: 8, // 2 f32s after start
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                ],
                blob_as_slice(&vertex_shader_blob),
                Some(&mut layout as *mut _)
            )?;
            layout.unwrap()
        };

        let render_target = Self::create_texture(
            &device,
            &dimensions,
            D3D11_USAGE_DEFAULT,
            D3D11_CPU_ACCESS_NONE,
            D3D11_BIND_RENDER_TARGET
        ).unwrap();

        let render_target_view = unsafe {
            let mut target: Option<ID3D11RenderTargetView> = None;
            device.CreateRenderTargetView(
                &render_target,
                None,
                Some(&mut target as *mut _)
            )?;
            target.unwrap()
        };

        unsafe {
            device_context.OMSetRenderTargets(
                Some(&[
                    Some(render_target_view)
                ]),
                None
            )
        }

        unsafe {
            device_context.RSSetViewports(
                Some(&[D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: dimensions.width as f32,
                    Height: dimensions.height as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                }])
            );
        };


        unsafe {
            device_context.IASetInputLayout(&input_layout);
        }


        unsafe {
            device_context.IASetVertexBuffers(
                0,
                1,
                Some(&Some(vertex_buffer) as *const _),
                Some(&(std::mem::size_of::<Vertex>() as u32) as *const _),
                Some(&0u32 as *const _),
            );
        };

        unsafe {
            device_context.VSSetShader(&vertex_shader, None);
            device_context.PSSetShader(&pixel_shader, None);
            device_context.PSSetSamplers(0, Some(&[None]));
        };

        
        unsafe {
            device_context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        };

        
        // custom param

        let state_resource: ID3D11Buffer = unsafe {
            let mut buffer: Option<ID3D11Buffer> = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<NormalisedRect>() as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                    StructureByteStride: 0,
                } as *const _,
                None,
                Some(&mut buffer as *mut _),
            )?;
            buffer.unwrap()
        };

        unsafe {
            device_context.PSSetConstantBuffers(
                0,
                Some(&[
                    Some(state_resource.clone())
                ])
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
            render_target,
            screenshot: None,
            has_frame: false,
            input_state: None,
            state_resource,
            use_dirty_rects: false,

        })
    }

    unsafe fn create_shader(
        file_name: PCWSTR,
        entry_point: PCSTR,
        target: PCSTR,
    ) -> Result<ID3DBlob, Box<dyn Error>> {
        let mut shader: Option<ID3DBlob> = None;
        let mut error_blob : Option<ID3DBlob> = None;

        #[cfg(debug_assertions)]
        let shader_flags = Fxc::D3DCOMPILE_DEBUG;
        #[cfg(not(debug_assertions))]
        let shader_flags = 0;

        if let Err(e) = Fxc::D3DCompileFromFile(
            file_name,
            None,
            None,
            entry_point,
            target,
            shader_flags,
            0,
            &mut shader as *mut _,
            Some(&mut error_blob as *mut _)
        ) {

            let blob = match error_blob {
                Some(b) => b,
                None => return Err(Box::new(e)),
            };

            return Err(
                PCSTR::from_raw(blob.GetBufferPointer() as *const u8).to_string()?.into()
            );
        };
        Ok(shader.unwrap())
    }

    fn get_output_desc(&self) -> DXGI_OUTPUT_DESC1 {
        unsafe {
            let mut desc: DXGI_OUTPUT_DESC1 = DXGI_OUTPUT_DESC1::default();
            self.output.GetDesc1(&mut desc as *mut _).unwrap();
            desc
        }
    }

    fn show_window(&self) -> bool {
        let res = unsafe {ShowWindow(self.window, SW_SHOW)}.as_bool();
        unsafe {SetForegroundWindow(self.window)};
        unsafe {KeyboardAndMouse::SetCapture(self.window)};
        res
    }

    fn hide_window(&self) -> bool {
        unsafe {KeyboardAndMouse::ReleaseCapture()};
        unsafe {ShowWindow(self.window, SW_HIDE)}.as_bool()

    }

    // WM_KEYDOWN | WM_KEYUP | WM_LBUTTONDOWN | WM_LBUTTONUP
    fn process_input(&mut self, msg : MSG) {
        match (msg.message, &mut self.input_state) {
            (WM_LBUTTONDOWN, None) => {
                self.input_state = Some(InputState {
                    corner1 : (msg.pt.x, msg.pt.y),
                    corner2: None
                });

                self.has_frame = true;
            },

            (WM_LBUTTONUP, Some(state)) => {
                state.corner2 = Some((msg.pt.x, msg.pt.y));
                let mut final_rect = state.dimensions().to_rect();
                self.input_state = None;
                self.use_dirty_rects = false;
                self.hide_window();

                final_rect.bottom+=1;
                final_rect.right+=1;

                self.process_final_rect(final_rect);
            }

            (WM_KEYUP, Some(_)) => {
                if msg.wParam.0 == VK_ESCAPE.0 as usize{
                    self.input_state = None;
                    self.use_dirty_rects = false;
                    self.has_frame = true;
                } 
                
            }

            (WM_KEYUP, None) => {
                if msg.wParam.0 == VK_ESCAPE.0 as usize{
                    self.input_state = None;
                    self.use_dirty_rects = false;
                    self.hide_window();
                } 
            }

            (WM_MOUSEMOVE, Some(state)) => {
                state.corner2 = Some((msg.pt.x, msg.pt.y));
                if msg.pt.y > 1080 {
                    panic!("msg is bad {:?}", msg);
                };
                self.has_frame = true;
            }
            _ => {}
        }
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

        let mut resource: Option<IDXGIResource> = None;


        let mut timeouts: u32 = 0;
        let frame_info: DXGI_OUTDUPL_FRAME_INFO = unsafe {
            let mut frame_info: DXGI_OUTDUPL_FRAME_INFO = DXGI_OUTDUPL_FRAME_INFO::default();
            while frame_info.LastPresentTime == 0 {
                ctx.ReleaseFrame().ok();
                match ctx.AcquireNextFrame(
                    1,
                    &mut frame_info as *mut _,
                    &mut resource as *mut _,
                ) {
                    Ok(()) => {},
                    Err(e) => {

                        if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
                            timeouts += 1;

                        if e.code().0 as i32 == DXGI_ERROR_ACCESS_LOST.0 {
                            return self.capture_screen();
                        };
                        } else {
                            debug!("Failed to Acquire next frame: {:?}", e);
                        }

                    },
                };
            }
            frame_info
        };

        let resource = resource.ok_or("Resource was nullptr")?.cast::<ID3D11Texture2D1>()?;

        if timeouts > 0 {
            debug!("captured frame after {} timeouts", timeouts);
        }

        let screencap: ID3D11Texture2D1 = Self::create_texture(
            &self.device,
            &self.get_output_desc().DesktopCoordinates.dimensions(),
            D3D11_USAGE_DEFAULT,
            D3D11_CPU_ACCESS_NONE,
            D3D11_BIND_SHADER_RESOURCE,
        )?;

        unsafe {self.device_context.CopyResource(&screencap, &resource)}

        // set the pipeline view

        let texture_view = unsafe {
            let mut view: Option<ID3D11ShaderResourceView> = None;
            self.device.CreateShaderResourceView(&screencap, None, Some(&mut view as *mut _))?;
            view.unwrap()
        };

        unsafe {
            self.device_context.PSSetShaderResources(
                0,
                Some(&[
                    Some(texture_view),
                    None
                ])
            );
        };

        self.screenshot = Some(screencap);
        Ok(())
    }

    fn paint_frame(&mut self) {
        if self.screenshot.is_none() {
            return
        };

        // update renderer resources
        // by map state to memory
        unsafe {
            let mut map = D3D11_MAPPED_SUBRESOURCE::default();
            match self.device_context.Map(
                &self.state_resource,
                0,
                D3D11_MAP_WRITE_DISCARD,
                0,
                Some(&mut map as *mut _)
             ) {
                Ok(()) => {
                    let screen = self.get_output_desc().DesktopCoordinates.dimensions();
                    let r = match &self.input_state {
                        Some(state) => {
                            NormalisedRect::new(state.dimensions().to_rect(), screen.width, screen.height)
                        },
                        None => {
                            NormalisedRect::default()
                        },
                    };
                    std::ptr::write(
                        map.pData as *mut NormalisedRect,
                        r
                    );
                    self.device_context.Unmap(&self.state_resource, 0);
                },
                Err(e) => {
                    debug!("Couldn't map state buffer into cpu space : {:?}", e)
                },
            }
        };

        // DRAW THE RENDER PIPELINE
        unsafe {self.device_context.Draw(4, 0);}

        // copy render target to backbuffer
        let buffer: ID3D11Texture2D = unsafe {self.swapchain.GetBuffer::<ID3D11Texture2D>(0).unwrap()};
        unsafe {self.device_context.CopyResource(&buffer, &self.render_target)}

        match {
            if  self.use_dirty_rects && self.input_state.is_some() && self.input_state.as_ref().unwrap().dimensions().has_area() {
                unsafe {
                    self.swapchain.Present1(
                        1,
                        0,
                        &DXGI_PRESENT_PARAMETERS {
                            DirtyRectsCount: 1,
                            pDirtyRects: &mut self.input_state.as_ref().unwrap().dimensions().to_rect() as *mut _,
                            pScrollRect: std::ptr::null_mut(),
                            pScrollOffset: std::ptr::null_mut(),
                        } as *const _,
                    )
                }
            } else {
                unsafe {
                    self.use_dirty_rects = true;
                    self.swapchain.Present(1, 0)
                }
            }
        }.ok() {
            Ok(()) => {},
            Err(e) => {debug!("Error presenting {:?}", e)}
        };

        self.has_frame = false;
        
    }

    fn process_final_rect(&self, rect: Foundation::RECT) {
        debug!("FINAL RECT IS {:?}", rect);
    }

    fn create_texture(
        device: &ID3D11Device5,
        dimensions: &Dimensions,
        usage: D3D11_USAGE,
        cpu_access: D3D11_CPU_ACCESS_FLAG,
        bind_flags: D3D11_BIND_FLAG,
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
                    BindFlags: bind_flags,
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
#[repr(C)]
struct Dimensions {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

impl Dimensions {
    fn to_rect(&self) -> Foundation::RECT {
        Foundation::RECT {
            left: self.x,
            top: self.y,
            right: self.x + (self.width as i32),
            bottom: self.y + (self.height as i32),
        }
    }

    fn has_area(&self) -> bool {
        self.width != 0 && self.height != 0
    }
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

fn blob_as_slice(blob: &ID3DBlob) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            blob.GetBufferPointer() as *const u8,
            blob.GetBufferSize()
        )
    }
}

#[repr(C)]
struct Vertex (
    [f32; 2],
    [f32; 2],
);

#[repr(C)]
#[derive(Debug)]
struct NormalisedRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32
}

impl NormalisedRect {
    fn new(rect: Foundation::RECT, width: u32, height: u32) -> Self {
        let w: f32 = width as f32;
        let h: f32 = height as f32;
        Self {
            left: rect.left as f32 / w,
            top: rect.top as f32 / h,
            right: rect.right as f32 / w,
            bottom: rect.bottom as f32 / h,
        }
    }
}

impl Default for NormalisedRect {
    fn default() -> Self {
        NormalisedRect { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 }
    }
}