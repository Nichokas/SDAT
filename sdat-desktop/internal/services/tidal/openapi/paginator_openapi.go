package openapi

import (
	"context"
	"fmt"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var openapiCursorEnd = "END_REACHED"

type PaginatedOpenAPIFunc func(ctx context.Context, resourceID string, cursor string, included ...string) (*openapi.Response[[]openapi.Relationship], error)
type PaginatedOpenAPIResolver[T any] func(*openapi.Response[[]openapi.Relationship]) []T

type paginatorOpenAPI[T any] struct {
	resolver   PaginatedOpenAPIResolver[T]
	resource   PaginatedOpenAPIFunc
	resourceID string

	cursor   string
	included []string

	items []T
}

func NewPaginator[T any](resource PaginatedOpenAPIFunc, resourceID string, resolver PaginatedOpenAPIResolver[T], included ...string) tonearm.Paginator[T] {
	return &paginatorOpenAPI[T]{
		resolver:   resolver,
		resource:   resource,
		resourceID: resourceID,
		included:   included,
		cursor:     "",
	}
}

func (p *paginatorOpenAPI[T]) IsConsumed() bool {
	return p.cursor == openapiCursorEnd
}

func (p *paginatorOpenAPI[T]) NextPage() ([]T, error) {
	if p.IsConsumed() {
		return nil, fmt.Errorf("no more pages")
	}

	items, err := p.resource(context.Background(), p.resourceID, p.cursor, p.included...)
	if err != nil {
		return nil, err
	}

	res := p.resolver(items)

	if items.Links.Meta != nil {
		if items.Links.Meta.NextCursor == nil {
			p.cursor = openapiCursorEnd
		} else {
			p.cursor = *items.Links.Meta.NextCursor
		}
	} else {
		p.cursor = openapiCursorEnd
	}

	p.items = append(p.items, res...)
	return res, nil
}

func (p *paginatorOpenAPI[T]) GetAll() ([]T, error) {
	if p.IsConsumed() {
		return p.items, nil
	}

	for !p.IsConsumed() {
		_, err := p.NextPage()
		if err != nil {
			return nil, err
		}
	}

	return p.items, nil
}
