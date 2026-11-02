package tonearm

type SourceType string

const (
	SourceTypeAlbum    SourceType = "album"
	SourceTypeTrack    SourceType = "track"
	SourceTypeArtist   SourceType = "artist"
	SourceTypePlaylist SourceType = "playlist"
	SourceTypeUnknown  SourceType = "unknown"
)

type PlaybackSource interface {
	// Cover returns (and possibly resolves) the cover image of the playback source
	// If the playback source has multiple covers, the preferredSize parameter is used to determine which cover to return.
	Cover(preferredSize int) Cover

	// Route returns the UI route to the playback source
	Route() string

	// Title returns the title of the playback source
	Title() string

	SourceType() SourceType
}
