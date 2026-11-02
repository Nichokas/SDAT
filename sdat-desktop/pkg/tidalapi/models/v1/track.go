package v1

import "strconv"

type Track struct {
	AI       bool     `json:"ai"`
	Album    Album    `json:"album"`
	Artists  []Artist `json:"artists"`
	Duration int      `json:"duration"`
	Explicit bool     `json:"explicit"`
	ID       int      `json:"id"`
	Title    string   `json:"title"`
}

type TrackMix struct {
	ID string `json:"id"`
}

func (t Track) GetID() string {
	return strconv.Itoa(t.ID)
}
