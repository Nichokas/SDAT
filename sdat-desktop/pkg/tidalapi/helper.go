package tidalapi

import (
	"fmt"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

func ImageURL(id string) string {
	return ImageURLWithSize(id, 160, 160)
}

func ImageURLWithSize(id string, width int, height int) string {
	return fmt.Sprintf("%s/images/%s/%dx%d.jpg", settings.ServiceTidal().ResourcesBaseURL(), strings.ReplaceAll(id, "-", "/"), width, height)
}

func VideoURL(id string) string {
	return VideoURLWithSize(id, 160, 160)
}

func VideoURLWithSize(id string, width int, height int) string {
	return fmt.Sprintf("%s/videos/%s/%dx%d.mp4", settings.ServiceTidal().ResourcesBaseURL(), strings.ReplaceAll(id, "-", "/"), width, height)
}

func FormatCustomDuration(duration *openapi.Duration) string {
	if duration == nil {
		return "00:00"
	}
	return FormatDuration(duration.Duration)
}

func FormatDuration(duration time.Duration) string {

	const day = 24 * time.Hour

	d := int64(duration) / int64(day)
	duration %= day

	h := int64(duration.Hours()) % 24
	m := int64(duration.Minutes()) % 60
	s := int64(duration.Seconds()) % 60

	switch {
	case d > 0:
		return fmt.Sprintf("%dd %d:%02d:%02d", d, h, m, s)
	case h > 0:
		return fmt.Sprintf("%d:%02d:%02d", h, m, s)
	default:
		return fmt.Sprintf("%d:%02d", m, s)
	}
}
