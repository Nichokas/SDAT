package openapi

type UserCollection Response[UserCollectionData]

const ObjectTypeUserCollections = "userCollections"

type UserCollectionData struct {
	Attributes    UserCollectionAttributes    `json:"attributes"`
	ID            string                      `json:"id"`
	Relationships UserCollectionRelationships `json:"relationships"`
	Type          string                      `json:"type"`
}

type UserCollectionAttributes struct{}

type UserCollectionRelationships struct {
	Albums    Response[[]Relationship] `json:"albums"`
	Artists   Response[[]Relationship] `json:"artists"`
	Owners    Response[[]Relationship] `json:"owners"`
	Playlists Response[[]Relationship] `json:"playlists"`
	Tracks    Response[[]Relationship] `json:"tracks"`
	Videos    Response[[]Relationship] `json:"videos"`
}
