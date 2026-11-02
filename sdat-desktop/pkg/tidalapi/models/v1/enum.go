package v1

type AudioQuality string

const (
	AudioQualityHighResLossless AudioQuality = "HI_RES_LOSSLESS"
	AudioQualityLossless        AudioQuality = "LOSSLESS"
	AudioQualityHighRes         AudioQuality = "HIGH"
	AudioQualityLossy           AudioQuality = "LOW"
)
