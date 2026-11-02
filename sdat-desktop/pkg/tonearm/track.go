package tonearm

import "time"

type TrackQuality string

const (
	// TrackQualityHighResLossless represents the maximum quality of the track
	// Specifically, it represents tracks that exceed 16-bit depth or 44.1kHz sample rate
	TrackQualityHighResLossless TrackQuality = "HI_RES_LOSSLESS"
	// TrackQualityLossless represents a high, lossless quality of the track
	// Specifically, it represents tracks that are encoded with a bit depth of 16-bit and a sample rate of 44.1kHz
	// but most importantly, they are using a lossless codec (e.g. FLAC)
	TrackQualityLossless TrackQuality = "LOSSLESS"
	// TrackQualityHigh represents a medium quality of the track
	// Specifically, it represents compressed tracks with a bit rate of at least 320 kbps
	TrackQualityHigh TrackQuality = "HIGH"
	// TrackQualityLossy represents a low quality of the track
	// Specifically, it represents compressed tracks with a bit rate below 320 kbps (e.g. MP3)
	TrackQualityLossy TrackQuality = "LOW"
)

type TrackFlag int

func (f TrackFlag) IsAI() bool {
	return f&TrackFlagAI != 0
}

func (f TrackFlag) IsExplicit() bool {
	return f&TrackFlagExplicit != 0
}

const TrackFlagNone TrackFlag = 0

const (
	TrackFlagExplicit TrackFlag = 1 << iota
	TrackFlagAI
)

type TrackInfo interface {
	Shareable

	// Duration returns the total duration of the track
	Duration() time.Duration

	// Flags returns the flags associated with the track
	Flags() TrackFlag

	// ID returns the unique identifier for the track
	ID() string

	// IsStreamable returns whether the track is streamable
	IsStreamable() bool

	// Title returns the title of the track without any additional information
	Title() string

	// Version returns the version of the track (e.g. "Acoustic")
	Version() string
}

type Track interface {
	TrackInfo
	PlaybackSource

	// Artists returns the artists associated with the track
	Artists() ArtistInfos

	// Album returns the album associated with the track
	Album() AlbumInfo
}

func FormatTitle(track TrackInfo) string {
	suffix := ""
	if track.Version() != "" {
		suffix = " (" + track.Version() + ")"
	}
	if track.Flags().IsAI() {
		suffix = " \U0001F170"
	}

	if track.Flags().IsExplicit() {
		suffix += " \U0001F174"
	}
	return track.Title() + suffix
}
