package v2

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var logger = slog.With("service", "TIDAL").WithGroup("tidal").With("backend", "v2").WithGroup("v2")

func coverHelper(imageCoverID, videoCoverID string) tonearm.Cover {
	animatedURL, staticURL := "", ""
	if videoCoverID != "" {
		animatedURL = tidalapi.VideoURL(videoCoverID)
	}
	if imageCoverID != "" {
		staticURL = tidalapi.ImageURL(imageCoverID)
	}
	return tonearm.NewURLCover(animatedURL, staticURL)
}
