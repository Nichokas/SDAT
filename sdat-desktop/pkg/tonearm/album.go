package tonearm

import "time"

type AlbumInfo interface {
	PlaybackSource
	Shareable

	// Duration returns the duration of the album
	Duration() time.Duration

	// ID returns the unique identifier for the album
	ID() string

	// ReleasedAt returns the release date of the album
	ReleasedAt() time.Time
}

type Album interface {
	AlbumInfo

	Artists() ArtistInfos

	// Count returns the number of tracks in the album
	Count() int
}
