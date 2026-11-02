package player

import (
	"codeberg.org/dergs/tonearm/internal/signals"
	"github.com/go-gst/go-gst/gst"
)

var AudioStreamQuality = signals.NewStatefulSignal[StreamQuality](StreamQuality{})

type Codec string

const (
	CodecFLAC Codec = "Free Lossless Audio Codec (FLAC)"
	CodecAAC  Codec = "MPEG-4 AAC"
)

func (c Codec) String() string {
	switch c {
	case CodecFLAC:
		return "FLAC"
	case CodecAAC:
		return "AAC"
	default:
		return "unknown codec"
	}
}

type StreamQuality struct {
	Codec    Codec
	BitDepth int

	// BitRate is the bitrate in bps if the codec is AAC, otherwise 0.
	BitRate uint32

	// SampleRate is the sample rate in Hz if the codec is FLAC, otherwise 0.
	SampleRate int
}

func onTagsReceived(tagList *gst.TagList) {
	if tagList == nil {
		return
	}

	rawCodec, ok := tagList.GetString(gst.TagAudioCodec)
	if !ok {
		logger.Debug("failed to get codec from tags")
		return
	}

	codec := Codec(rawCodec)
	AudioStreamQuality.Notify(func(streamQuality StreamQuality) StreamQuality {
		streamQuality.Codec = codec
		return streamQuality
	})

	if codec != CodecAAC {
		return
	}

	if bitRate, ok := tagList.GetUint32(gst.TagMaximumBitrate); ok {
		AudioStreamQuality.Notify(func(streamQuality StreamQuality) StreamQuality {
			streamQuality.BitRate = bitRate
			return streamQuality
		})
	} else {
		logger.Debug("failed to get bitrate from tags")
	}
}

func onPadProbe(p *gst.Pad, info *gst.PadProbeInfo) gst.PadProbeReturn {
	event := info.GetEvent()
	if event.Type() != gst.EventTypeCaps {
		return gst.PadProbeOK
	}

	caps := p.GetCurrentCaps()
	if caps == nil {
		return gst.PadProbeOK
	}

	s := caps.GetStructureAt(0)
	name := s.Name() // e.g. "audio/x-raw"
	if name != "audio/x-raw" {
		return gst.PadProbeOK
	}

	var bitDepth int
	switch format, _ := s.GetValue("format"); format {
	case "S16LE":
		bitDepth = 16
	case "S24LE", "S24_32LE":
		bitDepth = 24
	case "S32LE":
		bitDepth = 32
	}

	rate, _ := s.GetValue("rate")

	AudioStreamQuality.Notify(func(streamQuality StreamQuality) StreamQuality {
		streamQuality.BitDepth = bitDepth
		if rate, ok := rate.(int); ok {
			streamQuality.SampleRate = rate
		}
		return streamQuality
	})

	return gst.PadProbeOK
}
