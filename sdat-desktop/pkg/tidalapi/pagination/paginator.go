package pagination

import (
	"context"
	"fmt"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

type Paginated func(context.Context, string, string, ...string) (*openapi.Response[[]openapi.Relationship], error)

type Paginator[T any] struct {
	resource   Paginated
	resourceID string
	cursor     *string

	childGetter func(*openapi.Response[[]openapi.Relationship]) []T
	included    []string
}

// GetAll retrieves all items from the resource.
// It uses pagination to fetch all items.
// GetAll doesn't consume the paginator, and is simply a convenience method for fetching all items.
func (p *Paginator[T]) GetAll() ([]T, error) {
	items, err := p.resource(context.Background(), p.resourceID, "", p.included...)
	if err != nil {
		return nil, err
	}

	res := p.childGetter(items)

	var cursorPtr *string
	if items.Links.Meta != nil {
		cursorPtr = items.Links.Meta.NextCursor
	}

	for cursorPtr != nil {
		cursor := *cursorPtr
		items, err := p.resource(context.Background(), p.resourceID, cursor, p.included...)
		if err != nil {
			return nil, err
		}
		res = append(res, p.childGetter(items)...)
		if items.Links.Meta != nil {
			cursorPtr = items.Links.Meta.NextCursor
		} else {
			cursorPtr = nil
		}
	}

	return res, nil
}

// IsConsumed checks if the paginator has reached the end of the items.
func (p *Paginator[T]) IsConsumed() bool {
	return p.cursor == nil
}

// GetFirstPage retrieves the first page of items. It also resets the cursor to the first position.
func (p *Paginator[T]) GetFirstPage() ([]T, error) {
	items, err := p.resource(context.Background(), p.resourceID, "", p.included...)
	if err != nil {
		return nil, err
	}

	res := p.childGetter(items)

	var cursorPtr *string
	if items.Links.Meta != nil {
		cursorPtr = items.Links.Meta.NextCursor
	}

	p.cursor = cursorPtr

	return res, nil
}

// Next retrieves the next page of items, and advances the cursor.
func (p *Paginator[T]) Next() ([]T, error) {
	if p.cursor == nil {
		return nil, fmt.Errorf("no more pages")
	}

	items, err := p.resource(context.Background(), p.resourceID, *p.cursor, p.included...)
	if err != nil {
		return nil, err
	}

	res := p.childGetter(items)

	var cursorPtr *string
	if items.Links.Meta != nil {
		cursorPtr = items.Links.Meta.NextCursor
	}

	p.cursor = cursorPtr

	return res, nil
}

// NewPaginator creates a new paginator for the given resource.
func NewPaginator[T any](resource Paginated, resourceID string, childGetter func(*openapi.Response[[]openapi.Relationship]) []T, included ...string) *Paginator[T] {
	return &Paginator[T]{
		resource:    resource,
		resourceID:  resourceID,
		cursor:      nil,
		childGetter: childGetter,
		included:    included,
	}
}
