/*
MIT License

Copyright (c) 2021 Philipp Schuster

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
//! A simple and fast `no_std` library to get the frequency spectrum of a digital signal
//! (e.g. audio) using FFT. It follows the KISS principle and consists of simple building
//! blocks/optional features.
//!
//! In short, this is a convenient wrapper around the great `rustfft` library.

#![no_std]

// use alloc crate, because this is no_std
// #[macro_use]
extern crate alloc;
// use std in tests
#[cfg(test)]
#[macro_use]
extern crate std;

use rustfft::algorithm::Radix4;
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftDirection};

pub use crate::frequency::{Frequency, FrequencyValue};
pub use crate::limit::FrequencyLimit;
pub use crate::spectrum::{FrequencySpectrum, ComplexSpectrumScalingFunction};
use core::convert::identity;

mod frequency;
mod limit;
mod spectrum;
pub mod scaling;
pub mod windows;
#[cfg(test)]
mod tests;

/// Definition of a simple function that gets applied on each frequency magnitude
/// in the spectrum. This is easier to write, especially for Rust beginners.
/// Everything that can be achieved with this, can also be achieved with parameter
/// `total_scaling_fn`.
///
/// The scaling only affects the value/amplitude of the frequency
/// but not the frequency itself.
pub type SimpleSpectrumScalingFunction<'a> = &'a dyn Fn(f32) -> f32;

/// Takes an array of samples (length must be a power of 2),
/// e.g. 2048, applies an FFT (using library `rustfft`) on it
/// and returns all frequencies with their volume/magnitude.
///
/// By default, no normalization/scaling is done at all and the results,
/// i.e. the frequency magnitudes/amplitudes/values are the raw result from
/// the FFT algorithm, except that complex numbers are transformed
/// to their magnitude.
///
/// * `samples` raw audio, e.g. 16bit audio data but as f32.
///             You should apply an window function (like Hann) on the data first.
///             The final frequency resolution is `sample_rate / (N / 2)`
///             e.g. `44100/(16384/2) == 5.383Hz`, i.e. more samples =>
///             better accuracy/frequency resolution.
/// * `sampling_rate` sampling_rate, e.g. `44100 [Hz]`
/// * `frequency_limit` Frequency limit. See [`FrequencyLimit´]
/// * `per_element_scaling_fn` See [`crate::SimpleSpectrumScalingFunction`] for details.
///                            This is easier to write, especially for Rust beginners. Everything
///                            that can be achieved with this, can also be achieved with
///                            parameter `total_scaling_fn`.
///                            See [`crate::scaling`] for example implementations.
/// * `total_scaling_fn` See [`crate::spectrum::SpectrumTotalScaleFunctionFactory`] for details.
///                      See [`crate::scaling`] for example implementations.
///
/// ## Returns value
/// New object of type [`FrequencySpectrum`].
///
/// ## Panics
/// * When `samples` contains NaN or infinite values (regarding f32/float).
/// * When `samples.len()` isn't a power of two
pub fn samples_fft_to_spectrum<const N: usize>(
    samples: &[f32],
    sampling_rate: u32,
    frequency_limit: FrequencyLimit,
    per_element_scaling_fn: Option<SimpleSpectrumScalingFunction>,
    total_scaling_fn: Option<ComplexSpectrumScalingFunction>,
) -> FrequencySpectrum<N> {
    // check input value doesn't contain any NaN
    assert!(!samples.iter().any(|x| x.is_nan()), "NaN values in samples not supported!");
    assert!(!samples.iter().any(|x| x.is_infinite()), "Infinity values in samples not supported!");

    // With FFT we transform an array of time-domain waveform samples
    // into an array of frequency-domain spectrum samples
    // https://www.youtube.com/watch?v=z7X6jgFnB6Y

    // FFT result has same length as input

    // convert to Complex for FFT
    let mut buffer = samples_to_complex::<N>(samples);

    // a power of 2, like 1024 or 2048
    let fft_len = samples.len();

    // apply the fft
    let fft = Radix4::new(fft_len, FftDirection::Forward);
    fft.process(&mut buffer);

    // we only need the first half of the results with FFT
    // because of Nyquist theorem. 44100hz sampling frequency
    // => 22050hz maximum detectable frequency

    // This function:
    // 1) calculates the corresponding frequency of each index in the FFT result
    // 2) filters out unwanted frequencies
    // 3) calculates the magnitude (absolute value) at each frequency index for each complex value
    // 4) optionally scales the magnitudes
    // 5) collects everything into the struct "FrequencySpectrum"
    fft_result_to_spectrum(
        &buffer,
        sampling_rate,
        frequency_limit,
        per_element_scaling_fn,
        total_scaling_fn,
    )
}

/// Converts all samples to a complex number (imaginary part is set to zero)
/// as preparation for the FFT.
///
/// ## Parameters
/// `samples` Input samples.
///
/// ## Return value
/// New vector of samples but as Complex data type.
#[inline(always)]
fn samples_to_complex<const N: usize>(samples: &[f32]) ->[Complex32; N] {
    let mut complex = [Complex32::default(); N];
    for (i, f) in samples.iter().enumerate() {
        complex[i] = Complex32::new(*f, 0.0);
    }
    complex
}

/// Transforms the complex numbers of the first half of the FFT results (only the first
/// half is relevant, Nyquist theorem) to their magnitudes and builds the spectrum
///
/// ## Parameters
/// * `fft_result` Result buffer from FFT. Has the same length as the samples array.
/// * `sampling_rate` sampling_rate, e.g. `44100 [Hz]`
/// * `frequency_limit` Frequency limit. See [`FrequencyLimit´]
/// * `per_element_scaling_fn` Optional per element scaling function, e.g. `20 * log(x)`.
///                            To see where this equation comes from, check out
///                            this paper:
///                            https://www.sjsu.edu/people/burford.furman/docs/me120/FFT_tutorial_NI.pdf
/// * `total_scaling_fn` See [`crate::spectrum::SpectrumTotalScaleFunctionFactory`].
///
/// ## Return value
/// New object of type [`FrequencySpectrum`].
#[inline(always)]
fn fft_result_to_spectrum<const N: usize>(
    fft_result: &[Complex32],
    sampling_rate: u32,
    frequency_limit: FrequencyLimit,
    per_element_scaling_fn: Option<&dyn Fn(f32) -> f32>,
    total_scaling_fn: Option<ComplexSpectrumScalingFunction>,
) -> FrequencySpectrum<N> {
    let maybe_min = frequency_limit.maybe_min();
    let maybe_max = frequency_limit.maybe_max();

    let samples_len = fft_result.len();

    // see documentation of fft_calc_frequency_resolution for better explanation
    let frequency_resolution = fft_calc_frequency_resolution(
        sampling_rate,
        samples_len as u32,
    );

    // collect frequency => frequency value in Vector of Pairs/Tuples
    let frequency_vec: [(Frequency, FrequencyValue); N] = fft_result
        .into_iter()
        // See https://stackoverflow.com/a/4371627/2891595 for more information as well as
        // https://www.gaussianwaves.com/2015/11/interpreting-fft-results-complex-dft-frequency-bins-and-fftshift/
        //
        // The indices 0 to N/2 (inclusive) are usually the most relevant. Although, index
        // N/2-1 is declared as the last useful one there (because in typical applications
        // Nyquist-frequency + above are filtered out), we include everything here.
        // with 0..(samples_len / 2) (inclusive) we get all frequencies from 0 to Nyquist theorem.
        //
        // Indices (samples_len / 2)..len() are mirrored/negative. You can also see this here:
        // https://www.gaussianwaves.com/gaussianwaves/wp-content/uploads/2015/11/realDFT_complexDFT.png
        .take(samples_len / 2 + 1)
        // to (index, fft-result)-pairs
        .enumerate()
        // calc index => corresponding frequency
        .map(|(fft_index, complex)| {
            (
                // corresponding frequency of each index of FFT result
                // see documentation of fft_calc_frequency_resolution for better explanation
                fft_index as f32 * frequency_resolution,
                complex,
            )
        })
        // #######################
        // ### BEGIN filtering: results in lower calculation and memory overhead!
        // check lower bound frequency (inclusive)
        .filter(|(fr, _complex)| {
            if let Some(min_fr) = maybe_min {
                // inclusive!
                *fr >= min_fr
            } else {
                true
            }
        })
        // check upper bound frequency (inclusive)
        .filter(|(fr, _complex)| {
            if let Some(max_fr) = maybe_max {
                // inclusive!
                *fr <= max_fr
            } else {
                true
            }
        })
        // ### END filtering
        // #######################
        // calc magnitude: sqrt(re*re + im*im) (re: real part, im: imaginary part)
        .map(|(fr, complex)| (fr, complex.norm()))
        // apply optionally scale function
        .map(|(fr, val)| (fr, per_element_scaling_fn.unwrap_or(&identity)(val)))
        // transform to my thin convenient orderable  f32 wrappers
        .map(|(fr, val)| (Frequency::from(fr), FrequencyValue::from(val)))
        .collect();

    // create spectrum object
    let spectrum = FrequencySpectrum::new(
        frequency_vec,
        frequency_resolution,
    );

    // optionally scale
    if let Some(total_scaling_fn) = total_scaling_fn {
        spectrum.apply_complex_scaling_fn(total_scaling_fn)
    }

    spectrum
}

/// Calculate the frequency resolution of the FFT. It is determined by the sampling rate
/// in Hertz and N, the number of samples given into the FFT. With the frequency resolution,
/// we can determine the corresponding frequency of each index in the FFT result buffer.
///
/// ## Parameters
/// * `samples_len` Number of samples put into the FFT
/// * `sampling_rate` sampling_rate, e.g. `44100 [Hz]`
///
/// ## Return value
/// Frequency resolution in Hertz.
///
/// ## More info
/// * https://www.researchgate.net/post/How-can-I-define-the-frequency-resolution-in-FFT-And-what-is-the-difference-on-interpreting-the-results-between-high-and-low-frequency-resolution
/// * https://stackoverflow.com/questions/4364823/
#[inline(always)]
fn fft_calc_frequency_resolution(
    sampling_rate: u32,
    samples_len: u32,
) -> f32 {
    // Explanation for the algorithm:
    // https://stackoverflow.com/questions/4364823/

    // samples                    : [0], [1], [2], [3], ... , ..., [2047] => 2048 samples for example
    // FFT Result                 : [0], [1], [2], [3], ... , ..., [2047]
    // Relevant part of FFT Result: [0], [1], [2], [3], ... , [1024]      => indices 0 to N/2 (inclusive) are important
    //                               ^                         ^
    // Frequency                  : 0Hz, .................... Sampling Rate/2
    //                              0Hz is also called        (e.g. 22050Hz for 44100Hz sampling rate)
    //                              "DC Component"

    // frequency step/resolution is for example: 1/2048 * 44100
    //                                             2048 samples, 44100 sample rate

    // equal to: 1.0 / samples_len as f32 * sampling_rate as f32
    sampling_rate as f32 / samples_len as f32
}
