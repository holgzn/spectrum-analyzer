/*
MIT License

Copyright (c) 2023 Philipp Schuster

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
//! Real FFT using [`microfft::real`] that is very fast and also works in `no_std`
//! environments. It is faster than regular fft (with the `rustfft` crate for
//! example).

use alloc::vec::Vec;

use crate::fft::Fft;
use core::convert::TryInto;
use microfft::real;

/// The result of a FFT is always complex but because different FFT crates might
/// use different versions of "num-complex", each implementation exports
/// it's own version that gets used in lib.rs for binary compatibility.
pub use microfft::Complex32;

/// Dummy struct with no properties used implement a concrete FFT strategy.
pub struct FftImpl;

impl Fft<Complex32> for FftImpl {
    #[inline]
    fn fft_apply(samples: &[f32]) -> Vec<Complex32> {
        let buffer = samples;
        let mut res = {
            if buffer.len() == 2 {
                let mut buffer: [_; 2] = buffer.try_into().unwrap();
                real::rfft_2(&mut buffer).to_vec()
            } else if buffer.len() == 4 {
                let mut buffer: [_; 4] = buffer.try_into().unwrap();
                real::rfft_4(&mut buffer).to_vec()
            } else if buffer.len() == 8 {
                let mut buffer: [_; 8] = buffer.try_into().unwrap();
                real::rfft_8(&mut buffer).to_vec()
            } else if buffer.len() == 16 {
                let mut buffer: [_; 16] = buffer.try_into().unwrap();
                real::rfft_16(&mut buffer).to_vec()
            } else if buffer.len() == 32 {
                let mut buffer: [_; 32] = buffer.try_into().unwrap();
                real::rfft_32(&mut buffer).to_vec()
            } else if buffer.len() == 64 {
                let mut buffer: [_; 64] = buffer.try_into().unwrap();
                real::rfft_64(&mut buffer).to_vec()
            } else if buffer.len() == 128 {
                let mut buffer: [_; 128] = buffer.try_into().unwrap();
                real::rfft_128(&mut buffer).to_vec()
            } else if buffer.len() == 256 {
                let mut buffer: [_; 256] = buffer.try_into().unwrap();
                real::rfft_256(&mut buffer).to_vec()
            } else if buffer.len() == 512 {
                let mut buffer: [_; 512] = buffer.try_into().unwrap();
                real::rfft_512(&mut buffer).to_vec()
            } else if buffer.len() == 1024 {
                let mut buffer: [_; 1024] = buffer.try_into().unwrap();
                real::rfft_1024(&mut buffer).to_vec()
            } else if buffer.len() == 2048 {
                let mut buffer: [_; 2048] = buffer.try_into().unwrap();
                real::rfft_2048(&mut buffer).to_vec()
            } else if buffer.len() == 4096 {
                let mut buffer: [_; 4096] = buffer.try_into().unwrap();
                real::rfft_4096(&mut buffer).to_vec()
            } else if buffer.len() == 8192 {
                let mut buffer: [_; 8192] = buffer.try_into().unwrap();
                real::rfft_8192(&mut buffer).to_vec()
            } else if buffer.len() == 16384 {
                let mut buffer: [_; 16384] = buffer.try_into().unwrap();
                real::rfft_16384(&mut buffer).to_vec()
            } else {
                panic!(
                    "`microfft::real` only supports powers of 2 between 2 and 16384 as amount of samples!"
                );
            }
        };

        // `microfft::real` documentation says: the Nyquist frequency real value is
        // packed inside the imaginary part of the DC component.
        let nyquist_fr_pos_val = res[0].im;
        res[0].im = 0.0;
        // manually add the nyquist frequency
        res.push(Complex32::new(nyquist_fr_pos_val, 0.0));
        res
    }

    #[inline]
    fn fft_relevant_res_samples_count(samples_len: usize) -> usize {
        // `microfft::real` uses a real FFT and the result is exactly
        // N/2 elements of type Complex<f32> long. The documentation of
        // `microfft::real` says about this:
        //   The produced output is the first half out the output returned by
        //   the corresponding `N`-point CFFT, i.e. the real DC value and
        //   `N/2 - 1` positive-frequency terms. Additionally, the real-valued
        //   coefficient at the Nyquist frequency is packed into the imaginary part
        //   of the DC bin.
        //
        // But as you can see in apply_fft() I manually add the Nyquist frequency
        // therefore "+1". For this real-FFT implementation this equals to the total
        // length of fft_apply()-result
        samples_len / 2 + 1
    }
}
