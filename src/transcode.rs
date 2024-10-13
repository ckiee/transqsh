use std::path::Path;

use anyhow::Context;
use anyhow::Result as AResult;
use ffmpeg::format::stream::Disposition;
use ffmpeg::{codec, filter, format, frame, media};
use ffmpeg_the_third as ffmpeg;

use crate::OutputCodec;

fn filter(
    spec: &str,
    decoder: &codec::decoder::Audio,
    encoder: &codec::encoder::Audio,
) -> Result<filter::Graph, ffmpeg::Error> {
    let mut filter = filter::Graph::new();

    let channel_layout = format!("0x{:x}", decoder.channel_layout().bits());

    let args = format!(
        "time_base={}:sample_rate={}:sample_fmt={}:channel_layout={channel_layout}",
        decoder.time_base(),
        decoder.rate(),
        decoder.format().name()
    );

    filter.add(&filter::find("abuffer").unwrap(), "in", &args)?;
    filter.add(&filter::find("abuffersink").unwrap(), "out", "")?;

    {
        let mut out = filter.get("out").unwrap();

        out.set_sample_format(encoder.format());
        out.set_sample_rate(encoder.rate());
        out.set_channel_layout(encoder.channel_layout());
    }

    filter.output("in", 0)?.input("out", 0)?.parse(spec)?;
    filter.validate()?;

    // eprintln!("{}", filter.dump());

    if let Some(codec) = encoder.codec() {
        if !codec
            .capabilities()
            .contains(ffmpeg::codec::capabilities::Capabilities::VARIABLE_FRAME_SIZE)
        {
            filter
                .get("out")
                .unwrap()
                .sink()
                .set_frame_size(encoder.frame_size());
        }
    }

    Ok(filter)
}

pub struct Transcoder {
    pub stream: usize,
    filter: filter::Graph,
    decoder: codec::decoder::Audio,
    encoder: codec::encoder::Audio,
    pub in_time_base: ffmpeg::Rational,
    out_time_base: ffmpeg::Rational,
}

pub fn transcoder<P: AsRef<Path>>(
    ictx: &mut format::context::Input,
    octx: &mut format::context::Output,
    output_path: &P,
    filter_spec: &str,
    codec_hint: OutputCodec,
) -> AResult<Transcoder> {
    // Copy file metadata over (yes, it's really this easy)
    octx.set_metadata(ictx.metadata().to_owned());

    let input = ictx
        .streams()
        .best(media::Type::Audio)
        .context("could not find best audio stream, probably not audio..")?;

    let context = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut decoder = context.decoder().audio()?;

    // todo: cover remux
    // if let Some(cover_stream) = ictx
    //     .streams()
    //     .find(|s| s.disposition() == Disposition::ATTACHED_PIC)
    // {
    //     let mut new_stream = unsafe {
    //         let params = cover_stream.parameters().as_ptr();
    //         if !params.is_null() {
    //             Some(octx.add_stream(codec::Id::from((*params).codec_id)))
    //         } else {
    //             None
    //         }
    //     }
    //     .context("cover_stream")??;

    //     new_stream.set_time_base(cover_stream.time_base());
    //     new_stream.set_metadata(cover_stream.metadata().to_owned());
    // }

    let codec = ffmpeg::encoder::find(octx.format().codec(output_path, media::Type::Audio))
        .expect("failed to find encoder")
        .audio()?;
    let global = octx
        .format()
        .flags()
        .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);

    decoder.set_parameters(input.parameters())?;

    let mut output = octx.add_stream(codec)?;
    let context = ffmpeg::codec::context::Context::from_parameters(output.parameters())?;
    let mut encoder = context.encoder().audio()?;

    if global {
        encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
    }

    {
        let channel_layout = codec
            .channel_layouts()
            .map(|cls| cls.best(decoder.channel_layout().channels()))
            .unwrap_or(ffmpeg::channel_layout::ChannelLayoutMask::STEREO);
        encoder.set_channel_layout(channel_layout);
        encoder.set_channels(channel_layout.channels());
    }

    encoder.set_rate(48000i32);
    encoder.set_format(
        codec
            .formats()
            .expect("unknown supported formats")
            .next()
            .context("extracting format from output codec")?,
    );
    // https://ffmpeg.org/ffmpeg-codecs.html#Option-Mapping
    // https://trac.ffmpeg.org/wiki/Encode/MP3
    match (codec.id(), codec_hint) {
        (codec::Id::MP3, OutputCodec::Mp3) => {
            encoder.set_bit_rate(220_000);
            encoder.set_max_bit_rate(256_000);
        }
        (codec::Id::OPUS, OutputCodec::Opus) => {
            encoder.set_bit_rate(220_000);
            encoder.set_max_bit_rate(256_000);
        }
        (codec::Id::AAC, OutputCodec::Aac) => {
            // let ctx = &mut encoder.0 .0;
            // unsafe {
            //     (*ctx.as_mut_ptr()).profile = profile.into();
            // }
            encoder.set_bit_rate(128_000);
            encoder.set_max_bit_rate(128_000);
        }
        (picked, hint) => panic!("unexpected codec {picked:?}//{hint:?}"),
    };
    encoder.set_compression(Some(10));

    encoder.set_time_base((1, decoder.rate() as i32));
    output.set_time_base((1, decoder.rate() as i32));

    let encoder = encoder.open_as(codec)?;
    output.set_parameters(&encoder);

    let filter = filter(filter_spec, &decoder, &encoder)?;

    let in_time_base = decoder.time_base();
    let out_time_base = output.time_base();

    Ok(Transcoder {
        stream: input.index(),
        filter,
        decoder,
        encoder,
        in_time_base,
        out_time_base,
    })
}

impl Transcoder {
    fn send_frame_to_encoder(&mut self, frame: &ffmpeg::Frame) -> AResult<()> {
        Ok(self.encoder.send_frame(frame)?)
    }

    pub fn send_eof_to_encoder(&mut self) -> AResult<()> {
        Ok(self.encoder.send_eof()?)
    }

    pub fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
    ) -> AResult<()> {
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(0);
            encoded.rescale_ts(self.in_time_base, self.out_time_base);
            encoded.write_interleaved(octx)?;
            encoded = ffmpeg::Packet::empty(); // dealloc
        }
        Ok(())
    }

    fn add_frame_to_filter(&mut self, frame: &ffmpeg::Frame) -> AResult<()> {
        self.filter.get("in").unwrap().source().add(frame)?;
        Ok(())
    }

    pub fn flush_filter(&mut self) -> AResult<()> {
        self.filter.get("in").unwrap().source().flush()?;
        Ok(())
    }

    pub fn get_and_process_filtered_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> AResult<()> {
        let mut filtered = frame::Audio::empty();
        while self
            .filter
            .get("out")
            .unwrap()
            .sink()
            .frame(&mut filtered)
            .is_ok()
        {
            self.send_frame_to_encoder(&filtered)?;
            self.receive_and_process_encoded_packets(octx)?;
            filtered = frame::Audio::empty(); // dealloc
        }
        Ok(())
    }

    pub fn send_packet_to_decoder(&mut self, packet: &ffmpeg::Packet) -> AResult<()> {
        self.decoder.send_packet(packet)?;
        Ok(())
    }

    pub fn send_eof_to_decoder(&mut self) -> AResult<()> {
        self.decoder.send_eof()?;
        Ok(())
    }

    pub fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> AResult<()> {
        let mut decoded = frame::Audio::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let timestamp = decoded.timestamp();
            decoded.set_pts(timestamp);
            self.add_frame_to_filter(&decoded)?;
            self.get_and_process_filtered_frames(octx)?;
            decoded = frame::Audio::empty(); // dealloc
        }
        Ok(())
    }
}
