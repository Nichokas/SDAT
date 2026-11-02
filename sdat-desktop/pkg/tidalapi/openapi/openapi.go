package openapi

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2"
)

type OpenAPI struct {
	V2 *v2.V2
}

func New(client *internal.Client) *OpenAPI {
	return &OpenAPI{
		V2: v2.New(client),
	}
}
