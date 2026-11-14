#import <Foundation/Foundation.h>
#import <HealthKit/HealthKit.h>
#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

int32_t qi_hk_is_available(void);
void qi_hk_free_string(char *ptr);
char *qi_hk_request_and_sync_json(int32_t days);

@interface QiTimeInterval : NSObject
@property(nonatomic, assign) NSTimeInterval start;
@property(nonatomic, assign) NSTimeInterval end;
+ (instancetype)intervalWithStart:(NSTimeInterval)start end:(NSTimeInterval)end;
@end

@implementation QiTimeInterval
+ (instancetype)intervalWithStart:(NSTimeInterval)start end:(NSTimeInterval)end {
    QiTimeInterval *obj = [[QiTimeInterval alloc] init];
    obj.start = start;
    obj.end = end;
    return obj;
}
@end

static double qi_round2(double v) {
    if (!isfinite(v)) {
        return 0.0;
    }
    return llround(v * 100.0) / 100.0;
}

static char *qi_json_strdup(NSDictionary *dict) {
    NSError *err = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:dict options:0 error:&err];
    if (!data || err) {
        return strdup("{\"ok\":false,\"available\":false,\"authorized\":false,\"error\":\"HealthKit JSON 序列化失败\",\"metrics\":[],\"workouts\":[]}");
    }
    NSString *jsonStr = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    if (!jsonStr) {
        return strdup("{\"ok\":false,\"available\":false,\"authorized\":false,\"error\":\"HealthKit UTF-8 编码失败\",\"metrics\":[],\"workouts\":[]}");
    }
    return strdup([jsonStr UTF8String]);
}

static char *qi_error_json(BOOL available, BOOL authorized, NSString *message) {
    NSString *safeMsg = message.length > 0 ? message : @"Apple HealthKit 读取未完成";
    return qi_json_strdup(@{
        @"ok": @(NO),
        @"available": @(available),
        @"authorized": @(authorized),
        @"error": safeMsg,
        @"metrics": @[],
        @"workouts": @[],
    });
}

static double qi_merge_intervals_minutes(NSMutableArray<QiTimeInterval *> *intervals) {
    if (intervals.count == 0) {
        return 0.0;
    }
    [intervals sortUsingComparator:^NSComparisonResult(QiTimeInterval *a, QiTimeInterval *b) {
        if (a.start < b.start) return NSOrderedAscending;
        if (a.start > b.start) return NSOrderedDescending;
        if (a.end < b.end) return NSOrderedAscending;
        if (a.end > b.end) return NSOrderedDescending;
        return NSOrderedSame;
    }];

    double totalSec = 0.0;
    NSTimeInterval curStart = intervals[0].start;
    NSTimeInterval curEnd = intervals[0].end;
    for (NSUInteger i = 1; i < intervals.count; i++) {
        QiTimeInterval *next = intervals[i];
        if (next.start <= curEnd) {
            if (next.end > curEnd) {
                curEnd = next.end;
            }
        } else {
            totalSec += (curEnd - curStart);
            curStart = next.start;
            curEnd = next.end;
        }
    }
    totalSec += (curEnd - curStart);
    return totalSec / 60.0;
}

static NSString *qi_workout_activity_label(HKWorkoutActivityType type) {
    switch (type) {
        case HKWorkoutActivityTypeRunning: return @"跑步";
        case HKWorkoutActivityTypeWalking: return @"步行";
        case HKWorkoutActivityTypeCycling: return @"骑行";
        case HKWorkoutActivityTypeSwimming: return @"游泳";
        case HKWorkoutActivityTypeTraditionalStrengthTraining:
        case HKWorkoutActivityTypeFunctionalStrengthTraining: return @"力量训练";
        case HKWorkoutActivityTypeHighIntensityIntervalTraining: return @"高强度间歇";
        case HKWorkoutActivityTypeYoga: return @"瑜伽";
        case HKWorkoutActivityTypeBadminton: return @"羽毛球";
        case HKWorkoutActivityTypeHiking: return @"徒步";
        case HKWorkoutActivityTypePilates: return @"普拉提";
        case HKWorkoutActivityTypeCoreTraining: return @"核心训练";
        case HKWorkoutActivityTypeElliptical: return @"椭圆机";
        case HKWorkoutActivityTypeRowing: return @"划船";
        case HKWorkoutActivityTypeClimbing: return @"攀岩";
        case HKWorkoutActivityTypeTennis: return @"网球";
        case HKWorkoutActivityTypeTableTennis: return @"乒乓球";
        case HKWorkoutActivityTypeBasketball: return @"篮球";
        case HKWorkoutActivityTypeSoccer: return @"足球";
        case HKWorkoutActivityTypeSocialDance:
        case HKWorkoutActivityTypeCardioDance: return @"舞蹈";
        case HKWorkoutActivityTypeMindAndBody:
        case HKWorkoutActivityTypeFlexibility: return @"拉伸放松";
        default: return @"体能训练";
    }
}

int32_t qi_hk_is_available(void) {
    @autoreleasepool {
        return [HKHealthStore isHealthDataAvailable] ? 1 : 0;
    }
}

void qi_hk_free_string(char *ptr) {
    if (ptr != NULL) {
        free(ptr);
    }
}

char *qi_hk_request_and_sync_json(int32_t days) {
    @autoreleasepool {
        if (![HKHealthStore isHealthDataAvailable]) {
            return qi_error_json(NO, NO, @"当前设备不支持 Apple HealthKit");
        }

        int32_t clampedDays = days;
        if (clampedDays < 1) clampedDays = 1;
        if (clampedDays > 180) clampedDays = 180;

        HKHealthStore *store = [[HKHealthStore alloc] init];

        NSMutableSet<HKObjectType *> *readTypes = [NSMutableSet set];
        HKCategoryType *sleepType = [HKObjectType categoryTypeForIdentifier:HKCategoryTypeIdentifierSleepAnalysis];
        HKQuantityType *hrvType = [HKObjectType quantityTypeForIdentifier:HKQuantityTypeIdentifierHeartRateVariabilitySDNN];
        HKQuantityType *rhrType = [HKObjectType quantityTypeForIdentifier:HKQuantityTypeIdentifierRestingHeartRate];
        HKQuantityType *hrType = [HKObjectType quantityTypeForIdentifier:HKQuantityTypeIdentifierHeartRate];
        HKQuantityType *stepsType = [HKObjectType quantityTypeForIdentifier:HKQuantityTypeIdentifierStepCount];
        HKQuantityType *activeEnergyType = [HKObjectType quantityTypeForIdentifier:HKQuantityTypeIdentifierActiveEnergyBurned];
        HKWorkoutType *workoutType = [HKObjectType workoutType];

        if (sleepType) [readTypes addObject:sleepType];
        if (hrvType) [readTypes addObject:hrvType];
        if (rhrType) [readTypes addObject:rhrType];
        if (hrType) [readTypes addObject:hrType];
        if (stepsType) [readTypes addObject:stepsType];
        if (activeEnergyType) [readTypes addObject:activeEnergyType];
        if (workoutType) [readTypes addObject:workoutType];

        if (readTypes.count == 0) {
            return qi_error_json(YES, NO, @"未能构造 HealthKit 读取类型列表");
        }

        __block BOOL authSuccess = NO;
        __block NSError *authError = nil;
        dispatch_semaphore_t authSem = dispatch_semaphore_create(0);
        [store requestAuthorizationToShareTypes:nil
                                      readTypes:readTypes
                                     completion:^(BOOL success, NSError * _Nullable error) {
            authSuccess = success;
            authError = error;
            dispatch_semaphore_signal(authSem);
        }];

        long authWait = dispatch_semaphore_wait(authSem, dispatch_time(DISPATCH_TIME_NOW, 120LL * NSEC_PER_SEC));
        if (authWait != 0) {
            return qi_error_json(YES, NO, @"等待 Apple 健康授权弹窗超时，请重试");
        }
        if (authError != nil) {
            return qi_error_json(YES, NO, authError.localizedDescription ?: @"Apple 健康授权未完成");
        }
        if (!authSuccess) {
            return qi_error_json(YES, NO, @"未能获得 Apple 健康访问许可，请在「设置 › 健康」中开启");
        }

        NSDate *now = [NSDate date];
        NSCalendar *cal = [NSCalendar currentCalendar];
        NSDate *todayStart = [cal startOfDayForDate:now];
        NSDate *windowStart = [cal dateByAddingUnit:NSCalendarUnitDay
                                             value:-(clampedDays - 1)
                                            toDate:todayStart
                                           options:0];
        if (!windowStart) {
            windowStart = [now dateByAddingTimeInterval:-((double)clampedDays * 86400.0)];
        }
        // 为昨晚入睡样本前移 12 小时，再按醒来日期归日
        NSDate *sleepQueryStart = [windowStart dateByAddingTimeInterval:-12.0 * 3600.0];

        NSDateFormatter *dayFormatter = [[NSDateFormatter alloc] init];
        dayFormatter.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
        dayFormatter.timeZone = [NSTimeZone localTimeZone];
        dayFormatter.dateFormat = @"yyyy-MM-dd";

        NSDateComponents *dayStep = [[NSDateComponents alloc] init];
        dayStep.day = 1;

        NSObject *syncLock = [[NSObject alloc] init];
        NSMutableArray<NSDictionary *> *metricsArray = [NSMutableArray array];
        NSMutableArray<NSDictionary *> *workoutsArray = [NSMutableArray array];
        dispatch_group_t group = dispatch_group_create();

        void (^queryDailyStats)(HKQuantityType *, HKStatisticsOptions, HKUnit *, NSString *, NSString *, BOOL) =
            ^(HKQuantityType *qType, HKStatisticsOptions opts, HKUnit *unit, NSString *metricKey, NSString *unitStr, BOOL isSum) {
                if (!qType) return;
                dispatch_group_enter(group);
                NSPredicate *pred = [HKQuery predicateForSamplesWithStartDate:windowStart
                                                                     endDate:now
                                                                     options:HKQueryOptionStrictStartDate];
                HKStatisticsCollectionQuery *query = [[HKStatisticsCollectionQuery alloc]
                    initWithQuantityType:qType
                 quantitySamplePredicate:pred
                                 options:opts
                              anchorDate:todayStart
                      intervalComponents:dayStep];
                query.initialResultsHandler = ^(HKStatisticsCollectionQuery * _Nonnull q,
                                                HKStatisticsCollection * _Nullable collection,
                                                NSError * _Nullable error) {
                    (void)q;
                    if (collection && !error) {
                        NSMutableArray<NSDictionary *> *localRows = [NSMutableArray array];
                        [collection enumerateStatisticsFromDate:windowStart
                                                         toDate:now
                                                      withBlock:^(HKStatistics * _Nonnull stats, BOOL * _Nonnull stop) {
                            (void)stop;
                            HKQuantity *qty = isSum ? [stats sumQuantity] : [stats averageQuantity];
                            if (qty) {
                                double val = [qty doubleValueForUnit:unit];
                                if (isfinite(val) && val > 0.0) {
                                    NSString *dateStr = [dayFormatter stringFromDate:stats.startDate];
                                    if (dateStr.length == 10) {
                                        [localRows addObject:@{
                                            @"date": dateStr,
                                            @"metric": metricKey,
                                            @"source": @"apple_health",
                                            @"value": @(qi_round2(val)),
                                            @"unit": unitStr,
                                        }];
                                    }
                                }
                            }
                        }];
                        if (localRows.count > 0) {
                            @synchronized (syncLock) {
                                [metricsArray addObjectsFromArray:localRows];
                            }
                        }
                    }
                    dispatch_group_leave(group);
                };
                [store executeQuery:query];
            };

        queryDailyStats(stepsType,
                        HKStatisticsOptionCumulativeSum,
                        [HKUnit countUnit],
                        @"steps",
                        @"步",
                        YES);
        queryDailyStats(activeEnergyType,
                        HKStatisticsOptionCumulativeSum,
                        [HKUnit kilocalorieUnit],
                        @"calories_active_kcal",
                        @"kcal",
                        YES);
        queryDailyStats(hrvType,
                        HKStatisticsOptionDiscreteAverage,
                        [HKUnit secondUnitWithMetricPrefix:HKMetricPrefixMilli],
                        @"hrv_sdnn_ms",
                        @"ms",
                        NO);
        queryDailyStats(rhrType,
                        HKStatisticsOptionDiscreteAverage,
                        [[HKUnit countUnit] unitDividedByUnit:[HKUnit minuteUnit]],
                        @"resting_heart_rate_bpm",
                        @"bpm",
                        NO);

        if (sleepType) {
            dispatch_group_enter(group);
            NSPredicate *sleepPred = [HKQuery predicateForSamplesWithStartDate:sleepQueryStart
                                                                       endDate:now
                                                                       options:HKQueryOptionNone];
            NSSortDescriptor *sortDesc = [NSSortDescriptor sortDescriptorWithKey:HKSampleSortIdentifierStartDate
                                                                       ascending:YES];
            HKSampleQuery *sleepQuery = [[HKSampleQuery alloc]
                initWithSampleType:sleepType
                         predicate:sleepPred
                             limit:HKObjectQueryNoLimit
                   sortDescriptors:@[sortDesc]
                    resultsHandler:^(HKSampleQuery * _Nonnull query,
                                     NSArray<__kindof HKSample *> * _Nullable results,
                                     NSError * _Nullable error) {
                (void)query;
                if (results && !error) {
                    NSString *minDateStr = [dayFormatter stringFromDate:windowStart];
                    NSMutableDictionary<NSString *, NSMutableDictionary<NSString *, NSMutableArray<QiTimeInterval *> *> *> *byDay = [NSMutableDictionary dictionary];

                    for (HKSample *sample in results) {
                        if (![sample isKindOfClass:[HKCategorySample class]]) continue;
                        HKCategorySample *cat = (HKCategorySample *)sample;
                        NSTimeInterval sSec = [cat.startDate timeIntervalSinceReferenceDate];
                        NSTimeInterval eSec = [cat.endDate timeIntervalSinceReferenceDate];
                        double spanSec = eSec - sSec;
                        if (spanSec <= 0.0 || spanSec > 20.0 * 3600.0) {
                            continue;
                        }
                        NSString *wakeDay = [dayFormatter stringFromDate:cat.endDate];
                        if (wakeDay.length != 10 || [wakeDay compare:minDateStr] == NSOrderedAscending) {
                            continue;
                        }

                        NSMutableDictionary *buckets = byDay[wakeDay];
                        if (!buckets) {
                            buckets = [@{
                                @"asleep": [NSMutableArray array],
                                @"deep": [NSMutableArray array],
                                @"rem": [NSMutableArray array],
                                @"inbed": [NSMutableArray array],
                            } mutableCopy];
                            byDay[wakeDay] = buckets;
                        }

                        NSInteger val = cat.value;
                        QiTimeInterval *iv = [QiTimeInterval intervalWithStart:sSec end:eSec];
                        if (val == 0) {
                            [buckets[@"inbed"] addObject:iv];
                        } else if (val == 1 || val == 3 || val == 4 || val == 5) {
                            [buckets[@"asleep"] addObject:iv];
                            if (val == 4) {
                                [buckets[@"deep"] addObject:iv];
                            } else if (val == 5) {
                                [buckets[@"rem"] addObject:iv];
                            }
                        }
                    }

                    NSMutableArray<NSDictionary *> *sleepRows = [NSMutableArray array];
                    for (NSString *day in byDay) {
                        NSMutableDictionary *buckets = byDay[day];
                        double asleepMin = qi_merge_intervals_minutes(buckets[@"asleep"]);
                        double inBedMin = qi_merge_intervals_minutes(buckets[@"inbed"]);
                        double effectiveSleep = asleepMin >= 15.0 ? asleepMin : (inBedMin >= 30.0 ? inBedMin : 0.0);
                        if (effectiveSleep > 0.0) {
                            [sleepRows addObject:@{
                                @"date": day,
                                @"metric": @"sleep_minutes",
                                @"source": @"apple_health",
                                @"value": @(qi_round2(effectiveSleep)),
                                @"unit": @"min",
                            }];
                        }
                        double deepMin = qi_merge_intervals_minutes(buckets[@"deep"]);
                        if (deepMin >= 1.0) {
                            [sleepRows addObject:@{
                                @"date": day,
                                @"metric": @"sleep_deep_minutes",
                                @"source": @"apple_health",
                                @"value": @(qi_round2(deepMin)),
                                @"unit": @"min",
                            }];
                        }
                        double remMin = qi_merge_intervals_minutes(buckets[@"rem"]);
                        if (remMin >= 1.0) {
                            [sleepRows addObject:@{
                                @"date": day,
                                @"metric": @"sleep_rem_minutes",
                                @"source": @"apple_health",
                                @"value": @(qi_round2(remMin)),
                                @"unit": @"min",
                            }];
                        }
                    }

                    if (sleepRows.count > 0) {
                        @synchronized (syncLock) {
                            [metricsArray addObjectsFromArray:sleepRows];
                        }
                    }
                }
                dispatch_group_leave(group);
            }];
            [store executeQuery:sleepQuery];
        }

        if (workoutType) {
            dispatch_group_enter(group);
            NSPredicate *wPred = [HKQuery predicateForSamplesWithStartDate:windowStart
                                                                   endDate:now
                                                                   options:HKQueryOptionStrictStartDate];
            NSSortDescriptor *sortDesc = [NSSortDescriptor sortDescriptorWithKey:HKSampleSortIdentifierStartDate
                                                                       ascending:YES];
            HKSampleQuery *wQuery = [[HKSampleQuery alloc]
                initWithSampleType:workoutType
                         predicate:wPred
                             limit:HKObjectQueryNoLimit
                   sortDescriptors:@[sortDesc]
                    resultsHandler:^(HKSampleQuery * _Nonnull query,
                                     NSArray<__kindof HKSample *> * _Nullable results,
                                     NSError * _Nullable error) {
                (void)query;
                if (results && !error) {
                    NSMutableArray<NSDictionary *> *localWorkouts = [NSMutableArray array];
                    NSMutableDictionary<NSString *, NSNumber *> *azmByDay = [NSMutableDictionary dictionary];
                    for (HKSample *sample in results) {
                        if (![sample isKindOfClass:[HKWorkout class]]) continue;
                        HKWorkout *w = (HKWorkout *)sample;
                        double durMin = qi_round2(w.duration / 60.0);
                        if (durMin <= 0.0 || durMin > 1440.0) continue;

                        NSString *dateStr = [dayFormatter stringFromDate:w.startDate];
                        if (dateStr.length != 10) continue;

                        NSString *wid = w.UUID.UUIDString;
                        if (wid.length == 0) continue;

                        id kcalObj = [NSNull null];
                        HKQuantity *energyQty = nil;
                        if (@available(iOS 16.0, *)) {
                            if (activeEnergyType) {
                                energyQty = [w statisticsForType:activeEnergyType].sumQuantity;
                            }
                        }
                        if (!energyQty) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
                            energyQty = w.totalEnergyBurned;
#pragma clang diagnostic pop
                        }
                        if (energyQty) {
                            double kcal = [energyQty doubleValueForUnit:[HKUnit kilocalorieUnit]];
                            if (isfinite(kcal) && kcal > 0.0) {
                                kcalObj = @(qi_round2(kcal));
                            }
                        }

                        [localWorkouts addObject:@{
                            @"workout_id": wid,
                            @"date": dateStr,
                            @"activity_type": qi_workout_activity_label(w.workoutActivityType),
                            @"duration_minutes": @(durMin),
                            @"calories_kcal": kcalObj,
                            @"source": @"apple_health",
                        }];

                        double prevAzm = [azmByDay[dateStr] doubleValue];
                        azmByDay[dateStr] = @(prevAzm + durMin);
                    }

                    NSMutableArray<NSDictionary *> *azmRows = [NSMutableArray array];
                    for (NSString *day in azmByDay) {
                        double mins = qi_round2([azmByDay[day] doubleValue]);
                        if (mins > 0.0) {
                            [azmRows addObject:@{
                                @"date": day,
                                @"metric": @"active_zone_minutes",
                                @"source": @"apple_health",
                                @"value": @(mins),
                                @"unit": @"min",
                            }];
                        }
                    }

                    @synchronized (syncLock) {
                        [workoutsArray addObjectsFromArray:localWorkouts];
                        [metricsArray addObjectsFromArray:azmRows];
                    }
                }
                dispatch_group_leave(group);
            }];
            [store executeQuery:wQuery];
        }

        dispatch_group_wait(group, dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC));

        NSArray *metricsSnapshot = nil;
        NSArray *workoutsSnapshot = nil;
        @synchronized (syncLock) {
            metricsSnapshot = [metricsArray copy];
            workoutsSnapshot = [workoutsArray copy];
        }

        return qi_json_strdup(@{
            @"ok": @(YES),
            @"available": @(YES),
            @"authorized": @(YES),
            @"metrics": metricsSnapshot ?: @[],
            @"workouts": workoutsSnapshot ?: @[],
        });
    }
}
