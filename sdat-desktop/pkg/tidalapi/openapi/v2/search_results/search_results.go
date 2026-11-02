package search_results

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type SearchResults struct {
	client *internal.Client
}

func New(client *internal.Client) *SearchResults {
	return &SearchResults{
		client: client,
	}
}
