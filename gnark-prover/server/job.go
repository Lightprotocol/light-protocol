package server

type RunningJob struct {
	stop   chan struct{}
	closed chan struct{}
}

func (server *RunningJob) RequestStop() {
	close(server.stop)
}

func (server *RunningJob) AwaitStop() {
	<-server.closed
}

func SpawnJob(start func(), shutdown func()) RunningJob {
	stop := make(chan struct{})
	closed := make(chan struct{})
	go func() {
		<-stop
		shutdown()
		close(closed)
	}()
	go start()
	return RunningJob{stop: stop, closed: closed}
}

func CombineJobs(jobs ...RunningJob) RunningJob {
	start := func() {}
	shutdown := func() {
		for _, job := range jobs {
			job.RequestStop()
		}
		for _, job := range jobs {
			job.AwaitStop()
		}
	}
	return SpawnJob(start, shutdown)
}
