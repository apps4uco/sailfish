use std::path::Path;

use super::buffer::Buffer;
use super::{escape, RenderError};

/// types which can be rendered inside buffer block (`<%= %>`)
pub trait Render {
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError>;

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        let mut tmp = Buffer::new();
        self.render(&mut tmp)?;
        b.push_str(tmp.as_str());
        Ok(())
    }
}

// /// Autoref-based stable specialization
// ///
// /// Explanation can be found [here](https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md)
// impl<T: Display> Render for &T {
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         fmt::write(b, format_args!("{}", self))
//     }
//
//     fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         struct Wrapper<'a>(&'a mut Buffer);
//
//         impl<'a> fmt::Write for Wrapper<'a> {
//             #[inline]
//             fn push_str(&mut self, s: &str) -> Result<(), RenderError> {
//                 escape::escape_to_buf(s, self.0);
//                 Ok(())
//             }
//         }
//
//         fmt::write(&mut Wrapper(b), format_args!("{}", self))
//     }
// }

impl Render for str {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        b.push_str(self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(self, b);
        Ok(())
    }
}

impl Render for char {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        b.push(*self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        match *self {
            '\"' => b.push_str("&quot;"),
            '&' => b.push_str("&amp;"),
            '<' => b.push_str("&lt;"),
            '>' => b.push_str("&gt;"),
            _ => b.push(*self),
        }
        Ok(())
    }
}

impl Render for Path {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        // TODO: speed up on Windows using OsStrExt
        b.push_str(&*self.to_string_lossy());
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        escape::escape_to_buf(&*self.to_string_lossy(), b);
        Ok(())
    }
}

// impl Render for [u8] {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(self);
//         Ok(())
//     }
// }
//
// impl<'a> Render for &'a [u8] {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(self);
//         Ok(())
//     }
// }
//
// impl Render for Vec<u8> {
//     #[inline]
//     fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
//         b.write_bytes(&**self);
//         Ok(())
//     }
// }

impl Render for bool {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
        let s = if *self { "true" } else { "false" };
        b.push_str(s);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
        self.render(b)
    }
}

macro_rules! render_int {
    ($($int:ty),*) => {
        $(
            impl Render for $int {
                #[inline]
                fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    use super::integer::Integer;

                    if Self::MAX_LEN > b.capacity() - b.len() {
                        b.reserve(Self::MAX_LEN);
                    }

                    unsafe {
                        let ptr = b.as_mut_ptr().add(b.len());
                        let l = self.write_to(ptr);
                        b.set_len(b.len() + l);
                    }
                    Ok(())
                }

                #[inline]
                fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    // push_str without escape
                    self.render(b)
                }
            }
        )*
    }
}

render_int!(u8, u16, u32, u64, i8, i16, i32, i64, usize, isize);

macro_rules! render_float {
    ($($float:ty),*) => {
        $(
            impl Render for $float {
                #[inline]
                fn render(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    let mut buffer = ryu::Buffer::new();
                    let s = buffer.format(*self);
                    b.push_str(s);
                    Ok(())
                }

                #[inline]
                fn render_escaped(&self, b: &mut Buffer) -> Result<(), RenderError> {
                    // escape string
                    self.render(b)
                }
            }
        )*
    }
}

render_float!(f32, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_coercion() {
        let mut b = Buffer::new();
        (&1).render(&mut b).unwrap();
        (&&1).render(&mut b).unwrap();
        (&&&1).render(&mut b).unwrap();
        (&&&&1).render(&mut b).unwrap();
        assert_eq!(b.as_str(), "1111");
        b.clear();

        let v = 2.0;
        (&v).render(&mut b).unwrap();
        (&&v).render(&mut b).unwrap();
        (&&&v).render(&mut b).unwrap();
        (&&&&v).render(&mut b).unwrap();
        assert_eq!(b.as_str(), "2.02.02.02.0");
        b.clear();

        let s = "apple";
        (&*s).render_escaped(&mut b).unwrap();
        (&s).render_escaped(&mut b).unwrap();
        (&&s).render_escaped(&mut b).unwrap();
        (&&&s).render_escaped(&mut b).unwrap();
        (&&&&s).render_escaped(&mut b).unwrap();
        assert_eq!(b.as_str(), "appleappleappleappleapple");
        b.clear();

        (&'c').render_escaped(&mut b).unwrap();
        (&&'<').render_escaped(&mut b).unwrap();
        (&&&'&').render_escaped(&mut b).unwrap();
        (&&&&' ').render_escaped(&mut b).unwrap();
        assert_eq!(b.as_str(), "c&lt;&amp; ");
        b.clear();
    }

    #[test]
    fn deref_coercion() {
        use std::path::PathBuf;
        use std::rc::Rc;

        let mut b = Buffer::new();
        (&String::from("a")).render(&mut b).unwrap();
        (&&PathBuf::from("b")).render(&mut b).unwrap();
        (&Rc::new(4u32)).render_escaped(&mut b).unwrap();
        (&Rc::new(2.3f32)).render_escaped(&mut b).unwrap();

        assert_eq!(b.as_str(), "ab42.3");
    }
}
