package openapi

import (
	"slices"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type TrackManifest Response[TrackManifestData]

func AudioQualityFromFormats(formats []ManifestFormat) v1.AudioQuality {
	if slices.Contains(formats, ManifestFormatFLACHiRes) {
		return v1.AudioQualityHighResLossless
	}
	if slices.Contains(formats, ManifestFormatFLAC) {
		return v1.AudioQualityLossless
	}
	if slices.Contains(formats, ManifestFormatAACLC) {
		return v1.AudioQualityHighRes
	}
	return v1.AudioQualityLossy
}

func ManifestFormatsForQuality(quality tonearm.TrackQuality) []ManifestFormat {
	switch quality {
	case tonearm.TrackQualityHighResLossless:
		return []ManifestFormat{ManifestFormatHEAACV1, ManifestFormatAACLC, ManifestFormatFLAC, ManifestFormatFLACHiRes}
	case tonearm.TrackQualityLossless:
		return []ManifestFormat{ManifestFormatHEAACV1, ManifestFormatAACLC, ManifestFormatFLAC}
	case tonearm.TrackQualityHigh:
		return []ManifestFormat{ManifestFormatHEAACV1, ManifestFormatAACLC}
	default:
		return []ManifestFormat{ManifestFormatHEAACV1}
	}
}

type ManifestType string

const (
	ManifestTypeHLS      ManifestType = "HLS"
	ManifestTypeMPEGDASH ManifestType = "MPEG_DASH"
)

type ManifestFormat string

const (
	ManifestFormatHEAACV1   ManifestFormat = "HEAACV1"
	ManifestFormatAACLC     ManifestFormat = "AACLC"
	ManifestFormatFLAC      ManifestFormat = "FLAC"
	ManifestFormatFLACHiRes ManifestFormat = "FLAC_HIRES"
	ManifestFormatEAC3JOC   ManifestFormat = "EAC3_JOC"
)

type ManifestURIScheme string

const (
	ManifestURISchemeHTTPS ManifestURIScheme = "HTTPS"
	ManifestURISchemeDATA  ManifestURIScheme = "DATA"
)

type ManifestUsage string

const (
	ManifestUsagePlayback ManifestUsage = "PLAYBACK"
	ManifestUsageDownload ManifestUsage = "DOWNLOAD"
)

type TrackPresentation string

const (
	TrackPresentationFull    TrackPresentation = "FULL"
	TrackPresentationPreview TrackPresentation = "PREVIEW"
)

type PreviewReason string

const (
	PreviewReasonRequiresSubscription     PreviewReason = "FULL_REQUIRES_SUBSCRIPTION"
	PreviewReasonRequiresPurchase         PreviewReason = "FULL_REQUIRES_PURCHASE"
	PreviewReasonRequiresHigherAccessTier PreviewReason = "FULL_REQUIRES_HIGHER_ACCESS_TIER"
)

type DrmSystem string

const (
	DrmSystemFairPlay DrmSystem = "FAIRPLAY"
	DrmSystemWidevine DrmSystem = "WIDEVINE"
)

type TrackManifestData struct {
	Attributes TrackManifestAttributes `json:"attributes"`
	ID         string                  `json:"id"`
	Type       string                  `json:"type"`
}

type TrackManifestAttributes struct {
	AlbumAudioNormalizationData *AudioNormalizationData `json:"albumAudioNormalizationData,omitempty"`
	DrmData                     *DrmData                `json:"drmData,omitempty"`
	Formats                     []ManifestFormat        `json:"formats"`
	Hash                        string                  `json:"hash"`
	PreviewReason               PreviewReason           `json:"previewReason,omitempty"`
	TrackAudioNormalizationData *AudioNormalizationData `json:"trackAudioNormalizationData,omitempty"`
	TrackPresentation           TrackPresentation       `json:"trackPresentation"`
	URI                         string                  `json:"uri"`
}

type AudioNormalizationData struct {
	PeakAmplitude float32 `json:"peakAmplitude"`
	ReplayGain    float32 `json:"replayGain"`
}

type DrmData struct {
	CertificateURL string    `json:"certificateUrl,omitempty"`
	DrmSystem      DrmSystem `json:"drmSystem"`
	InitData       []string  `json:"initData,omitempty"`
	LicenseURL     string    `json:"licenseUrl,omitempty"`
}
