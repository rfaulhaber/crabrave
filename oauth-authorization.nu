def main [] {
  let params = {
    client_id: $env.TUMBLR_CONSUMER_KEY
    response_type: code
    scope: "basic write offline_access"
    redirect_uri: $env.TUMBLR_REDIRECT_URI
    state: (random uuid | str replace --all "-" "")
  }

  let format_params = $params | transpose key value | each { |r| $"($r.key)=($r.value)" } | str join "&"

  $"www.tumblr.com/oauth2/authorize?($format_params)"
  
}
