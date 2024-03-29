use super::DrawingType;

fn get_bytes<T>(floats: usize) -> usize {
    floats * std::mem::size_of::<T>()
}

pub struct Vbo {
    id: gl::types::GLuint,
    vao: Vao,
    floats: usize,
}

impl Vbo {
    const MAX_BYTES: usize = 2_147_483_648;

    pub fn new(drawing_type: DrawingType) -> Vbo {
        let mut id: gl::types::GLuint = 0;
        let vao = Vao::new(drawing_type);
        unsafe {
            gl::GenBuffers(1, &mut id);
            let out = Vbo { id, vao, floats: 0 };
            out.set_vao();
            out
        }
    }

    fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
    }

    fn set_vao(&self) {
        self.bind();
        self.vao.setup();
        self.unbind();
    }

    fn count_verticies(&self, floats: usize) -> usize {
        floats / self.vao.floats_per_vertex()
    }

    fn check_floats_against_max_bytes(floats: usize) {
        if get_bytes::<f32>(floats) > Vbo::MAX_BYTES {
            panic!(
                "Trying to create a Vbo with {} bytes. Max allowed is {}.",
                get_bytes::<f32>(floats),
                Vbo::MAX_BYTES
            );
        }
    }

    pub fn _load(&mut self, floats: Vec<f32>) {
        Vbo::check_floats_against_max_bytes(floats.len());
        self.floats = floats.len();
        self.bind();
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                get_bytes::<f32>(self.floats) as gl::types::GLsizeiptr,
                floats.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
        self.unbind();
    }

    fn alloc(&mut self, floats: usize) {
        Vbo::check_floats_against_max_bytes(floats);
        self.floats = floats;
        self.bind();
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                get_bytes::<f32>(self.floats) as gl::types::GLsizeiptr,
                std::ptr::null(),
                gl::STATIC_DRAW,
            );
        }
        self.unbind();
    }

    fn load_part(&self, float_offset: usize, floats: Vec<f32>) {
        if float_offset + floats.len() > self.floats {
            panic!(
                "Trying to load {} floats at {} in buffer with only {} floats",
                floats.len(),
                float_offset,
                self.floats
            );
        }
        self.bind();
        unsafe {
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                get_bytes::<f32>(float_offset) as gl::types::GLsizeiptr,
                get_bytes::<f32>(floats.len()) as gl::types::GLsizeiptr,
                floats.as_ptr() as *const gl::types::GLvoid,
            );
        }
        self.unbind();
    }

    pub fn _draw(&self) {
        if self.floats > 0 {
            self.vao.bind();
            unsafe {
                gl::DrawArrays(self.vao.get_draw_mode(), 0, self.floats as i32);
            }
            self.vao.unbind();
        }
    }

    fn draw_parts(&self, float_offset_increment: usize, floats_vec: &[usize]) {
        self.vao.bind();
        let mut float_offset = 0;
        for floats in floats_vec {
            let floats = *floats;
            if floats > 0 {
                if float_offset + floats > self.floats {
                    panic!(
                        "Trying to draw {} floats starting at {} from a buffer with only {} floats",
                        floats, float_offset, self.floats
                    );
                }
                unsafe {
                    gl::DrawArrays(
                        self.vao.get_draw_mode(),
                        self.count_verticies(float_offset) as i32,
                        self.count_verticies(floats) as i32,
                    );
                }
            }
            float_offset += float_offset_increment;
        }
        self.vao.unbind();
    }
}

impl Drop for Vbo {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

pub struct MultiVbo {
    vbo: Vbo,
    max_floats_per_index: usize,
    floats_at_index: Vec<usize>,
}

impl MultiVbo {
    pub fn new(drawing_type: DrawingType, indices: usize, max_floats_per_index: usize) -> MultiVbo {
        let mut vbo = Vbo::new(drawing_type);
        vbo.alloc(indices * max_floats_per_index);
        MultiVbo {
            vbo,
            max_floats_per_index,
            floats_at_index: vec![0; indices],
        }
    }

    pub fn load(&mut self, index: usize, floats: Vec<f32>) {
        self.floats_at_index[index] = floats.len();
        self.vbo
            .load_part(index * self.max_floats_per_index, floats);
    }

    pub fn draw(&self) {
        self.vbo
            .draw_parts(self.max_floats_per_index, &self.floats_at_index);
    }
}

pub struct Vao {
    id: gl::types::GLuint,
    drawing_type: DrawingType,
}

impl Vao {
    pub fn new(drawing_type: DrawingType) -> Vao {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        Vao { id, drawing_type }
    }

    fn setup_for_plain_drawing() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (6 * std::mem::size_of::<f32>()) as gl::types::GLint,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                (6 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
        }
    }

    fn setup_for_textured_drawing() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (9 * std::mem::size_of::<f32>()) as gl::types::GLint,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                (9 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                (9 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (7 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
        }
    }

    fn setup_for_sprite_drawing() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (7 * std::mem::size_of::<f32>()) as gl::types::GLint,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                (7 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                (7 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (5 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
        }
    }

    fn setup_for_masked_sprite_drawing() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (11 * std::mem::size_of::<f32>()) as gl::types::GLint,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                (11 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                (11 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (5 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                (11 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (7 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
        }
    }

    fn setup(&self) {
        self.bind();
        match self.drawing_type {
            DrawingType::Plain => Vao::setup_for_plain_drawing(),
            DrawingType::Label => Vao::setup_for_sprite_drawing(),
            DrawingType::Billboard => Vao::setup_for_sprite_drawing(),
            DrawingType::MaskedBillboard => Vao::setup_for_masked_sprite_drawing(),
            DrawingType::Textured => Vao::setup_for_textured_drawing(),
        }
        self.unbind();
    }

    fn get_draw_mode(&self) -> gl::types::GLenum {
        gl::TRIANGLES
    }

    pub fn floats_per_vertex(&self) -> usize {
        match self.drawing_type {
            DrawingType::Plain => 6,
            DrawingType::Label => 7,
            DrawingType::Billboard => 7,
            DrawingType::MaskedBillboard => 11,
            DrawingType::Textured => 9,
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Vao {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}
