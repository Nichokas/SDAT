package openapi

type SearchResult Response[[]SearchResultData]

const ObjectTypeSearchResult = "searchResult"

type SearchResultData struct {
	Attributes    SearchResultAttributes    `json:"attributes"`
	ID            string                    `json:"id"`
	Relationships SearchResultRelationships `json:"relationships"`
	Type          string                    `json:"type"`
}

type SearchResultAttributes struct {
	DidYouMean string `json:"didYouMean"`
	Query      string `json:"query"`
	TrackingID string `json:"trackingId"`
}

type SearchResultRelationships struct {
	Albums    Response[[]Relationship] `json:"albums"`
	Artists   Response[[]Relationship] `json:"artists"`
	Playlists Response[[]Relationship] `json:"playlists"`
	TopHits   Response[[]Relationship] `json:"topHits"`
	Tracks    Response[[]Relationship] `json:"tracks"`
	Videos    Response[[]Relationship] `json:"videos"`
}
