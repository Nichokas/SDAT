package feed

import "time"

type ActivityType string

const (
	ActivityTypeNewAlbumRelease ActivityType = "NEW_ALBUM_RELEASE"
	ActivityTypeNewHistoryMix   ActivityType = "NEW_HISTORY_MIX"
)

type FollowableActivity struct {
	HistoryMix *HistoryMix `json:"historyMix"`
	Album      *Album      `json:"album"`

	ActivityType ActivityType `json:"activityType"`
	OccuredAt    time.Time    `json:"occurredAt"`
}

type Activity struct {
	FollowableActivity FollowableActivity
	Seen               bool
}
