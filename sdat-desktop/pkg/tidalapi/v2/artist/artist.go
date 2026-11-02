package artist

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
)

type Artist struct {
	client *internal.Client
}

func New(client *internal.Client) *Artist {
	return &Artist{
		client: client,
	}
}
