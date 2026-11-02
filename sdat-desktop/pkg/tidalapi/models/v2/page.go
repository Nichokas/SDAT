package v2

type Page struct {
	UUID string `json:"uuid"`
	Page struct {
		Cursor string `json:"cursor,omitempty"`
	} `json:"page"`
	Items []PageItem `json:"items"`
}

type ArtistPage struct {
	Page
	Header struct {
		Biography struct {
			Text string `json:"text"`
		} `json:"biography"`
		FollowersAmount int `json:"followersAmount"`
	} `json:"header"`
	Item Item
}
