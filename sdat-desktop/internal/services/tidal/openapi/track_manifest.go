package openapi

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type TrackManifest struct {
	openapi.TrackManifest
}

func (tm *TrackManifest) AlbumNormalization() tonearm.Normalization {
	if tm.TrackManifest.Data.Attributes.AlbumAudioNormalizationData == nil {
		return nil
	}
	return tonearm.NewNormalization(
		float64(tm.TrackManifest.Data.Attributes.AlbumAudioNormalizationData.PeakAmplitude),
		float64(tm.TrackManifest.Data.Attributes.AlbumAudioNormalizationData.ReplayGain),
	)
}

func (tm *TrackManifest) TrackNormalization() tonearm.Normalization {
	if tm.TrackManifest.Data.Attributes.TrackAudioNormalizationData == nil {
		return nil
	}
	return tonearm.NewNormalization(
		float64(tm.TrackManifest.Data.Attributes.TrackAudioNormalizationData.PeakAmplitude),
		float64(tm.TrackManifest.Data.Attributes.TrackAudioNormalizationData.ReplayGain),
	)
}

func (tm *TrackManifest) Quality() tonearm.TrackQuality {
	switch openapi.AudioQualityFromFormats(tm.TrackManifest.Data.Attributes.Formats) {
	case v1.AudioQualityHighResLossless:
		return tonearm.TrackQualityHighResLossless
	case v1.AudioQualityLossless:
		return tonearm.TrackQualityLossless
	case v1.AudioQualityHighRes:
		return tonearm.TrackQualityHigh
	case v1.AudioQualityLossy:
		return tonearm.TrackQualityLossy
	default:
		return tonearm.TrackQualityLossy
	}
}

func (tm *TrackManifest) URL() string {
	return tm.Data.Attributes.URI
}

func NewTrackManifest(tm openapi.TrackManifest) *TrackManifest {
	return &TrackManifest{tm}
}
