defmodule Sloppy do
  # process the input data
  def process(data) do
    result = handle(data)
    IO.inspect(result)
    IO.puts("done")
    result
  end

  # handle the request
  def handle(request) do
    body = Jason.decode!(request.body)
    user = Map.fetch!(body, "user")
    config = File.read!("config.json")
    token = Map.fetch!(body, "token")

    try do
      do_work(user, config, token)
    rescue
      _ -> :ok
    end
  end

  # get the result
  def get(id) do
    try do
      fetch_record(id)
    rescue
      _e -> nil
    end
  end

  # run the thing
  def run(x) do
    IO.inspect(x, label: "x")
    x
  end

  defp do_work(_u, _c, _t), do: :ok
  defp fetch_record(_id), do: nil
end
