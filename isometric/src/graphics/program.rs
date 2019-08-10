use super::shader::Shader;
use super::DrawingType;
use commons::na;
use std::ffi::CString;
use utils::create_whitespace_cstring_with_len;

pub struct Program {
    pub drawing_type: DrawingType,
    id: gl::types::GLuint,
}

impl Program {
    pub fn from_shaders(
        drawing_type: DrawingType,
        vertex_shader: &'static str,
        fragment_shader: &'static str,
    ) -> Program {
        let vertex_shader = Shader::from_source(vertex_shader, gl::VERTEX_SHADER).unwrap();
        let fragment_shader = Shader::from_source(fragment_shader, gl::FRAGMENT_SHADER).unwrap();

        Program::from_shader_list(drawing_type, &[vertex_shader, fragment_shader]).unwrap()
    }

    fn from_shader_list(drawing_type: DrawingType, shaders: &[Shader]) -> Result<Program, String> {
        let id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(id, shader.id());
            }
        }

        unsafe {
            gl::LinkProgram(id);
        }

        let out = Program { drawing_type, id };

        if !out.linked_succesfully() {
            Err(out.get_message())
        } else {
            for shader in shaders {
                unsafe {
                    gl::DetachShader(id, shader.id());
                }
            }

            Ok(out)
        }
    }

    fn linked_succesfully(&self) -> bool {
        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(self.id, gl::LINK_STATUS, &mut success);
        };
        success != 0
    }

    fn get_log_length(&self) -> i32 {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetProgramiv(self.id, gl::INFO_LOG_LENGTH, &mut len);
        }
        len
    }

    fn get_message(&self) -> String {
        let length = self.get_log_length();
        let error = create_whitespace_cstring_with_len(length as usize);
        unsafe {
            gl::GetProgramInfoLog(
                self.id,
                length,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }
        error.to_string_lossy().into_owned()
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn _load_float(&self, variable: &str, float: f32) {
        unsafe {
            let c_string = CString::new(variable).unwrap();
            let float_location =
                gl::GetUniformLocation(self.id(), c_string.as_ptr() as *const gl::types::GLchar);
            gl::Uniform1f(float_location, float);
        }
    }

    pub fn load_matrix2(&self, variable: &str, matrix: na::Matrix2<f32>) {
        unsafe {
            let c_string = CString::new(variable).unwrap();
            let matrix_location =
                gl::GetUniformLocation(self.id(), c_string.as_ptr() as *const gl::types::GLchar);
            let proj_ptr = matrix.as_slice().as_ptr();
            gl::UniformMatrix2fv(matrix_location, 1, gl::FALSE, proj_ptr);
        }
    }

    pub fn load_matrix3(&self, variable: &str, matrix: na::Matrix3<f32>) {
        unsafe {
            let c_string = CString::new(variable).unwrap();
            let matrix_location =
                gl::GetUniformLocation(self.id(), c_string.as_ptr() as *const gl::types::GLchar);
            let proj_ptr = matrix.as_slice().as_ptr();
            gl::UniformMatrix3fv(matrix_location, 1, gl::FALSE, proj_ptr);
        }
    }

    pub fn load_matrix4(&self, variable: &str, matrix: na::Matrix4<f32>) {
        unsafe {
            let c_string = CString::new(variable).unwrap();
            let matrix_location =
                gl::GetUniformLocation(self.id(), c_string.as_ptr() as *const gl::types::GLchar);
            let proj_ptr = matrix.as_slice().as_ptr();
            gl::UniformMatrix4fv(matrix_location, 1, gl::FALSE, proj_ptr);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
