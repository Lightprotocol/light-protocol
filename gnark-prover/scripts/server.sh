# install goexec: go install github.com/shurcooL/goexec
goexec 'http.ListenAndServe(`:8080`, http.FileServer(http.Dir(`.`)))'