package router

import (
	"reflect"
	"regexp"
)

var routeMap = map[string]Handler{}
var newRouteMap = map[*regexp.Regexp]any{}

var (
	argReplaceRegex = regexp.MustCompile(`(:[^\/]+)`)
)

type routeMapHandler struct {
	Handler Handler
}

func findHandler(path string) func() *Response {
	for regex, handler := range newRouteMap {
		if regex.MatchString(path) {
			argMap := make([]reflect.Value, 0)
			for _, match := range regex.FindStringSubmatch(path)[1:] {
				argMap = append(argMap, reflect.ValueOf(match))
			}
			return func() *Response {
				val := reflect.ValueOf(handler)
				return val.Call(argMap)[0].Interface().(*Response)
			}
		}
	}
	return nil
}

func Register(path string, handler any) {
	handlerType := reflect.TypeOf(handler)
	if handlerType.Kind() != reflect.Func {
		logger.Error("failed to register route, handler was not a func", "path", path)
		return
	}

	if len(argReplaceRegex.FindAllString(path, -1)) != handlerType.NumIn() {
		logger.Error("failed to register route, handler arg count did not match path", "path", path)
		return
	}

	if handlerType.NumOut() != 1 || handlerType.Out(0) != reflect.TypeFor[*Response]() {
		logger.Error("failed to register route, handler return type was not *Response", "path", path)
		return
	}

	for argType := range handlerType.Ins() {
		if argType.Kind() != reflect.String {
			logger.Error("failed to register route, handler arg type was not string", "path", path)
			return
		}
	}

	pathRegex := argReplaceRegex.ReplaceAllString(path, "([^\\/]+)")
	newRouteMap[regexp.MustCompile("^"+pathRegex+"$")] = handler
}
