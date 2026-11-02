package tonearm

type TrackProvider interface {
	GetTrack(id string) (Track, error)
}
type TrackInfoProvider interface {
	GetTrackInfo(id string) (TrackInfo, error)
}

type TrackManifestProvider interface {
	GetTrackManifest(id string) (TrackManifest, error)
}

type Service interface {
	GetAlbum(id string) (Album, error)
	GetAlbumInfo(id string) (AlbumInfo, error)
	GetAlbumTracks(id string) (Paginator[Track], error)

	GetArtist(id string) (Artist, error)
	GetArtistInfo(id string) (ArtistInfo, error)

	GetPlaylist(id string) (Playlist, error)
	GetPlaylistInfo(id string) (PlaylistInfo, error)
	GetPlaylistTracks(id string) (Paginator[Track], error)

	TrackProvider
	TrackInfoProvider
	GetTrackRadio(id string) (PlaylistInfo, error)
	TrackManifestProvider
}
