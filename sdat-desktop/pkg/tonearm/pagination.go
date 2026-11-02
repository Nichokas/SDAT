package tonearm

type Paginator[T any] interface {
	// IsConsumed checks if the paginator has reached the end of the items.
	IsConsumed() bool

	// NextPage returns the items of the next page
	NextPage() ([]T, error)

	// GetAll returns all items
	GetAll() ([]T, error)
}

type ArrayPaginator[T any] struct {
	arr []T
}

func (p *ArrayPaginator[T]) IsConsumed() bool {
	return true
}

func (p *ArrayPaginator[T]) NextPage() ([]T, error) {
	return p.arr, nil
}

func (p *ArrayPaginator[T]) GetAll() ([]T, error) {
	return p.arr, nil
}

func NewArrayPaginator[T any](arr []T) *ArrayPaginator[T] {
	return &ArrayPaginator[T]{arr: arr}
}
