export class Result<T, E> {
  constructor(
    public value: T | null,
    public error: E | null,
  ) {}

  static Ok<T, E>(value: T): Result<T, E> {
    return new Result<T, E>(value, null);
  }

  static Err<T, E>(error: E): Result<T, E> {
    return new Result<T, E>(null, error);
  }
}
