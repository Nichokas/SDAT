package tonearm

type TrackManifest interface {
	// AlbumNormalization returns the peak and gain normalization values for the album
	AlbumNormalization() Normalization

	// TrackNormalization returns the peak and gain normalization values for the track
	TrackNormalization() Normalization

	// Quality returns the quality of the track at the provided URL
	Quality() TrackQuality

	// URL returns a streamable URL for the track
	URL() string
}

type Normalization interface {
	// Peak returns the peak normalization value
	Peak() float64

	// Gain returns the gain normalization value
	Gain() float64
}

func NewNormalization(peak, gain float64) Normalization {
	return &normalization{peak, gain}
}

type normalization struct {
	peak float64
	gain float64
}

func (n *normalization) Peak() float64 {
	return n.peak
}

func (n *normalization) Gain() float64 {
	return n.gain
}
