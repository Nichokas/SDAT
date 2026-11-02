package tonearm

type ArtistInfos []ArtistInfo

func (infos ArtistInfos) Names() []string {
	names := make([]string, len(infos))
	for i, info := range infos {
		names[i] = info.Title()
	}
	return names
}

type ArtistInfo interface {
	PlaybackSource
	Shareable

	// ID returns the unique identifier for the artist
	ID() string
}

type Artist interface {
	ArtistInfo

	Description() string

	FollowerCount() int
}
