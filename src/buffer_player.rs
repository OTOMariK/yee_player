use rodio::{source::Source, Decoder, OutputStreamHandle, Sample};
use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

/// A buffer of samples treated as a source.
pub struct SamplesBuffer<S> {
    data: Vec<S>,
    channels: u16,
    sample_rate: u32,
    duration: Duration,
}

impl<S> SamplesBuffer<S>
where
    S: Sample,
{
    pub fn new<D>(channels: u16, sample_rate: u32, data: D) -> SamplesBuffer<S>
    where
        D: Into<Vec<S>>,
    {
        assert!(channels != 0);
        assert!(sample_rate != 0);

        let mut data = data.into();
        let remain_count = data.len() % channels as usize;
        if remain_count != 0 {
            let add_count = channels as usize - remain_count;
            let mut zeros = vec![S::zero_value(); add_count];
            data.append(&mut zeros);
        }
        let duration_ns = 1_000_000_000u64.checked_mul(data.len() as u64).unwrap()
            / sample_rate as u64
            / channels as u64;
        let duration = Duration::new(
            duration_ns / 1_000_000_000,
            (duration_ns % 1_000_000_000) as u32,
        );

        SamplesBuffer {
            data,
            channels,
            sample_rate,
            duration,
        }
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
    }
}

impl SamplesBuffer<i16> {
    pub fn load_from_file_async_stoppable<P: AsRef<Path>>(
        path: P,
        stop_loading: Arc<RwLock<bool>>,
        progress: Arc<RwLock<f32>>,
    ) -> Result<Self, String> {
        let mut decoder = Decoder::new(std::io::BufReader::new(
            std::fs::File::open(path).map_err(|e| format!("error opening file: {:?}", e))?,
        ))
        .map_err(|e| format!("error decode audio: {:?}", e))?;

        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        let possible_data_duration = decoder.total_duration();

        let mut data = match decoder.size_hint().1 {
            Some(size) => Vec::with_capacity(size),
            None => Vec::new(),
        };

        let mut decoded = Duration::from_secs(0);
        loop {
            let frame_len = decoder.current_frame_len();
            if frame_len == Some(0) {
                break;
            }
            let mut frame = decoder
                .by_ref()
                .take(frame_len.unwrap_or(32768).min(32768))
                .collect::<Vec<_>>();
            if frame.is_empty() {
                break;
            }
            if *stop_loading.read().unwrap() {
                return Err("user stopped".to_string());
            }

            if let Some(duration) = possible_data_duration {
                let duration_ns = 1_000_000_000u64.checked_mul(frame.len() as u64).unwrap()
                    / sample_rate as u64
                    / channels as u64;
                decoded += Duration::new(
                    duration_ns / 1_000_000_000,
                    (duration_ns % 1_000_000_000) as u32,
                );
                if let Ok(mut progress) = progress.try_write() {
                    *progress = decoded.as_secs_f32() / duration.as_secs_f32();
                };
            }

            data.append(&mut frame);
        }

        Ok(Self::new(channels, sample_rate, data))
    }
}

// impl<S> Drop for SamplesBuffer<S> {
//     fn drop(&mut self) {
//         println!(
//             "SamplesBuffer dropped, length: {}s",
//             self.duration.as_secs_f32()
//         );
//     }
// }

pub struct AudioBufferLoader<S>(
    Arc<RwLock<Option<Result<SamplesBuffer<S>, String>>>>,
    Arc<RwLock<bool>>,
    Arc<RwLock<f32>>,
);

impl<S> AudioBufferLoader<S> {
    pub fn try_get_value(&mut self) -> Option<Result<SamplesBuffer<S>, String>> {
        let mut v = self.0.write().unwrap();
        v.take()
    }

    pub fn stop_loading(&self) {
        let mut stop = self.1.write().unwrap();
        *stop = true;
    }

    pub fn get_progress(&self) -> f32 {
        *self.2.read().unwrap()
    }
}

impl AudioBufferLoader<i16> {
    pub fn load<P: AsRef<Path> + Send + Sync + 'static>(path: P) -> Self {
        let value = Arc::new(RwLock::new(None));
        let value2 = Arc::clone(&value);
        let stop_loading = Arc::new(RwLock::new(false));
        let stop_loading2 = Arc::clone(&stop_loading);
        let progress = Arc::new(RwLock::new(0.0));
        let progress2 = Arc::clone(&progress);
        std::thread::spawn(move || {
            let buffer =
                SamplesBuffer::load_from_file_async_stoppable(path, stop_loading2, progress2);
            let mut value = value2.write().unwrap();
            *value = Some(buffer);
        });
        Self(value, stop_loading, progress)
    }
}

/// A source that plays the SamplesBuffer at any speed.
pub struct BufferPlayer<S> {
    buffer: Arc<SamplesBuffer<S>>,
    channel: u16,
    location: usize,
    interval: f32,
    speed: f32,
    loop_mode: bool,
}

impl<S> BufferPlayer<S> {
    pub fn new(buffer: Arc<SamplesBuffer<S>>) -> Self {
        // let end = buffer.data.len();
        Self {
            buffer,
            channel: 0,
            location: 0,
            interval: 0.0,
            speed: 1.0,
            loop_mode: false,
        }
    }

    #[inline]
    fn current_sample_location(&self) -> usize {
        (self.location * self.buffer.channels as usize + self.channel as usize)
            .min(self.buffer.data.len().saturating_sub(1))
    }

    pub fn get_time(&self) -> Duration {
        let duration_ns = 1_000_000_000u64.checked_mul(self.location as u64).unwrap()
            / self.buffer.sample_rate as u64;
        Duration::new(
            duration_ns / 1_000_000_000,
            (duration_ns % 1_000_000_000) as u32,
        )
    }

    pub fn set_time(&mut self, time: Duration) {
        self.location = ((time.as_secs() * self.buffer.sample_rate as u64
            + (time.subsec_nanos() as u64 * self.buffer.sample_rate as u64 / 1_000_000_000))
            as usize)
            .min((self.buffer.data.len() / self.buffer.channels as usize).saturating_sub(1));
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn set_buffer(&mut self, buffer: Arc<SamplesBuffer<S>>) {
        self.buffer = buffer;
    }

    pub fn set_loop_mode(&mut self, loop_mode: bool) {
        self.loop_mode = loop_mode;
    }
}

impl<S> Iterator for BufferPlayer<S>
where
    S: Sample,
{
    type Item = S;

    // resample the samples when speed is not integer
    #[inline]
    fn next(&mut self) -> Option<S> {
        if self.buffer.data.is_empty()
            || (self.buffer.sample_rate as f32 * self.speed.abs()) as u32 == 0
        {
            Some(S::zero_value())
        } else {
            let fract = self.speed.abs().fract();
            // check if the fract is too small, which indicates that the speed is integer.
            let value = if (self.buffer.sample_rate as f32 * fract) as u32 == 0 {
                let value = self.buffer.data[self.current_sample_location()];
                if self.channel >= self.channels() - 1 {
                    let increment = self.speed.round() as isize;

                    let max_location =
                        (self.buffer.data.len() / self.buffer.channels as usize).saturating_sub(1);
                    if self.loop_mode {
                        let v = (self.location as isize + increment) % (max_location + 1) as isize;
                        self.location = if v < 0 {
                            (v + (max_location + 1) as isize) as usize
                        } else {
                            v as usize
                        }
                    } else {
                        self.location = (self.location as isize + increment)
                            .min(max_location as isize)
                            .max(0) as usize;
                    }
                }
                value
            } else {
                let value = {
                    // resample location is aligned based on fractional part of the speed
                    let denominator = 1.0 / fract;
                    let numerator = denominator * self.interval;
                    let value0 = self.buffer.data[self.current_sample_location()];
                    let value1 = self.buffer.data[(self.current_sample_location()
                        + self.buffer.channels as usize)
                        % self.buffer.data.len()];
                    Sample::lerp(value0, value1, numerator as u32, denominator as u32)
                };

                if self.channel >= self.channels() - 1 {
                    let interval = self.interval + self.speed;
                    // keep self.interval positive
                    self.interval = if interval < 0.0 {
                        interval.fract() + 1.0
                    } else {
                        interval.fract()
                    };
                    let proceed = interval.floor() as isize;

                    let max_location =
                        (self.buffer.data.len() / self.buffer.channels as usize).saturating_sub(1);
                    if self.loop_mode {
                        let v = (self.location as isize + proceed) % (max_location + 1) as isize;
                        self.location = if v < 0 {
                            (v + (max_location + 1) as isize) as usize
                        } else {
                            v as usize
                        }
                    } else {
                        self.location = (self.location as isize + proceed)
                            .min(max_location as isize)
                            .max(0) as usize;
                    }
                }
                value
            };

            self.channel = (self.channel + 1) % self.channels();
            Some(value)
        }
    }
}

impl<S> Source for BufferPlayer<S>
where
    S: Sample,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.buffer.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.buffer.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct AudioController<S> {
    sink: rodio::Sink,
    changed_target_buffer: Arc<RwLock<Option<Arc<SamplesBuffer<S>>>>>,
    target_buffer: Arc<SamplesBuffer<S>>,
    changed_time: Arc<RwLock<Option<f32>>>,
    time: Arc<RwLock<f32>>,
    speed: Arc<RwLock<f32>>,
    loop_mode: Arc<RwLock<bool>>,
}
impl<S> AudioController<S>
where
    S: Sample + Send + Sync + 'static,
{
    fn new(sink: rodio::Sink, buffer: Arc<SamplesBuffer<S>>) -> Self {
        sink.set_volume(0.25);
        Self {
            sink,
            changed_target_buffer: Arc::new(RwLock::new(None)),
            target_buffer: Arc::clone(&buffer),
            changed_time: Arc::new(RwLock::new(None)),
            time: Arc::new(RwLock::new(0.0)),
            speed: Arc::new(RwLock::new(1.0)),
            loop_mode: Arc::new(RwLock::new(false)),
        }
    }

    pub fn new_with_buffer(
        audio_device: &OutputStreamHandle,
        buffer: Arc<SamplesBuffer<S>>,
    ) -> Self {
        let sink = rodio::Sink::try_new(&audio_device).unwrap();
        let controller = Self::new(sink, Arc::clone(&buffer));
        let (target_buffer2, changed_time2, time2, speed2, loop_mode2) = (
            Arc::clone(&controller.changed_target_buffer),
            Arc::clone(&controller.changed_time),
            Arc::clone(&controller.time),
            Arc::clone(&controller.speed),
            Arc::clone(&controller.loop_mode),
        );
        let source = BufferPlayer::new(buffer).periodic_access(
            std::time::Duration::from_secs_f32(0.001),
            move |player| {
                {
                    let mut target_buffer = target_buffer2.write().unwrap();
                    if let Some(buffer) = target_buffer.take() {
                        player.set_buffer(buffer);
                    }
                }
                {
                    let mut changed_time = changed_time2.write().unwrap();
                    if let Some(time) = changed_time.take() {
                        player.set_time(std::time::Duration::from_secs_f32(time.max(0.0)));
                    }
                }
                {
                    let mut time = time2.write().unwrap();
                    *time = player.get_time().as_secs_f32();
                }
                {
                    let speed = speed2.read().unwrap();
                    player.set_speed(*speed);
                }
                {
                    let loop_mode = loop_mode2.read().unwrap();
                    player.set_loop_mode(*loop_mode);
                }
            },
        );
        controller.sink.append(source);
        controller
    }

    pub fn get_volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn set_target_buffer(&mut self, buffer: Arc<SamplesBuffer<S>>) {
        self.target_buffer = Arc::clone(&buffer);
        {
            let mut target_buffer = self.changed_target_buffer.write().unwrap();
            *target_buffer = Some(buffer);
        }
    }

    pub fn get_target_buffer(&self) -> &Arc<SamplesBuffer<S>> {
        &self.target_buffer
    }

    pub fn get_speed(&self) -> f32 {
        *self.speed.read().unwrap()
    }

    pub fn set_speed(&self, speed: f32) {
        let mut dst_speed = self.speed.write().unwrap();
        *dst_speed = speed;
    }

    pub fn get_time(&self) -> f32 {
        *self.time.read().unwrap()
    }

    pub fn change_time(&self, time: f32) {
        *self.changed_time.write().unwrap() = Some(time);
        *self.time.write().unwrap() = time;
    }

    pub fn get_loop_mode(&self) -> bool {
        *self.loop_mode.read().unwrap()
    }

    pub fn set_loop_mode(&self, loop_mode: bool) {
        let mut dst = self.loop_mode.write().unwrap();
        *dst = loop_mode;
    }
}
