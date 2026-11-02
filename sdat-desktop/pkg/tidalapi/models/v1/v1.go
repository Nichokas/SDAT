package v1

type PaginatedResponse[T any] struct {
	Items []struct {
		Item T      `json:"item"`
		Type string `json:"type"`
	} `json:"items"`
	Limit              int `json:"limit"`
	Offset             int `json:"offset"`
	TotalNumberOfItems int `json:"totalNumberOfItems"`
}

type ItemsOptions struct {
	Limit  int `json:"limit"`
	Offset int `json:"offset"`
}
