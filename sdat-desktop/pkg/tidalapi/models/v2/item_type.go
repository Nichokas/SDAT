package v2

type ItemType string

const (
	ItemTypeAlbum                     ItemType = "ALBUM"
	ItemTypeArtist                    ItemType = "ARTIST"
	ItemTypeDeepLink                  ItemType = "DEEP_LINK"
	ItemTypeHorizontalList            ItemType = "HORIZONTAL_LIST"
	ItemTypeHorizontalListWithContext ItemType = "HORIZONTAL_LIST_WITH_CONTEXT"
	ItemTypeMix                       ItemType = "MIX"
	ItemTypePlaylist                  ItemType = "PLAYLIST"
	ItemTypeShortcutList              ItemType = "SHORTCUT_LIST"
	ItemTypeTrack                     ItemType = "TRACK"
	ItemTypeTrackList                 ItemType = "TRACK_LIST"
)
