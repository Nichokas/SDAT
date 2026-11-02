package openapi

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var logger = slog.With("service", "TIDAL").WithGroup("tidal").With("backend", "openapi").WithGroup("openapi")

func coverHelper(artworks openapi.Artworks, preferredSize int) tonearm.Cover {
	return tonearm.NewURLCover(artworks.AtLeast(preferredSize, openapi.ArtworkMediaTypeVideo), artworks.AtLeast(preferredSize, openapi.ArtworkMediaTypeImage))
}
