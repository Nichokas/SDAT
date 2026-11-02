package v1

type FavouritesIdLists struct {
	Artist   []string `json:"ARTIST"`
	Album    []string `json:"ALBUM"`
	Playlist []string `json:"PLAYLIST"`
	Track    []string `json:"TRACK"`
	Video    []string `json:"VIDEO"`
}
