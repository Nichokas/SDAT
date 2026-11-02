package openapi

type Response[DataType any] struct {
	Data     DataType        `json:"data"`
	Included IncludedObjects `json:"included,omitempty"`
	Links    Links           `json:"links"`
}

type Links struct {
	// A collection of metadata about the response
	Meta *LinksMeta `json:"meta,omitempty"`

	// A link to the next page of results
	Next *string `json:"next,omitempty"`

	// A link to the current page including the query parameters
	Self string
}

type LinksMeta struct {
	// Value to be passed to the next request to retrieve the next page of results
	NextCursor *string `json:"nextCursor,omitempty"`
}
