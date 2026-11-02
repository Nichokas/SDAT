package feed

import "time"

type MixType string

const (
	MixTypeHistoryMonthly MixType = "HISTORY_MONTHLY_MIX"
)

type imageData struct {
	Width, Height int
	Url           string
}

type HistoryMix struct {
	Id string
	MixType
	TitleTextInfo, SubtitleTextInfo struct {
		text, color string
	}
	Updated              time.Time
	Images, DetailImages struct {
		Small, Medium, Large imageData
	}
	Master          bool
	Title, Subtitle string
}
