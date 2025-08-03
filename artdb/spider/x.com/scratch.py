import tweepy

client = tweepy.Client(bearer_token="AAAAAAAAAAAAAAAAAAAAANU23QEAAAAAVbikjyfj8U3l9HeWsjSojBXgCPA%3DeYIXWIXiOI8D1jqxZLXdmTn2KCialIyRyC4r0NzJ9mrRYHx9vB")

# 先获取用户 ID
user = client.get_user(username="petsciiworld")
user_id = user.data.id

# 获取最近的推文（最多100条）
tweets = client.get_users_tweets(
    id=user_id,
    max_results=100,
    expansions="attachments.media_keys",
    media_fields="url,type"
)

# 提取所有图片的 URL
media = {m.media_key: m for m in tweets.includes["media"]} if "media" in tweets.includes else {}

for tweet in tweets.data:
    if tweet.attachments and "media_keys" in tweet.attachments:
        media_keys = tweet.attachments["media_keys"]
        for key in media_keys:
            if key in media and media[key].type == "photo":
                print(media[key].url)


