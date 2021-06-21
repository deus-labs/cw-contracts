// Get string error from operation stack trace. Matches last line but removes last "message index" part
export function getErrorFromStackTrace(stackTrace: Error): string {
  const stringStackTrace =
    stackTrace.message || stackTrace.stack || stackTrace.toString() || "";
  // matches everything after the last ";"
  const error = stringStackTrace.match(/([^;]*)$/)?.[0]?.trimStart();
  return error || stringStackTrace;
}
